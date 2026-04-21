//! `inbox` command: fetch every queued ciphertext for a UIN, process each
//! according to its MLS message type, and print decrypted application
//! messages.
//!
//! Uses `messages:fetchCiphertext` (a mutation — it atomically flips the
//! `delivered` flag so subsequent calls do not re-deliver the same bytes).
//!
//! Three message types are expected in this prototype:
//!   * Welcome — stage + join, then record the group ID in the sidecar
//!     table so later `send`/`inbox` invocations can find it.
//!   * PrivateMessage carrying ApplicationData — decrypt and print.
//!   * PrivateMessage carrying a StagedCommit — merge so our copy of the
//!     group's epoch advances (e.g. when a third party is added later).

use anyhow::{anyhow, Context, Result};
use openmls::prelude::{tls_codec::Deserialize as _, *};
use serde_json::json;

use crate::convex_client::{json_to_bytes, ConvexClient};
use crate::storage;

pub async fn run(my_uin: u64, db: &str) -> Result<()> {
    let provider = storage::open_provider(db)?;
    let client = ConvexClient::from_env()?;

    let result = client
        .mutation("messages:fetchCiphertext", json!({"recipientUin": my_uin}))
        .await
        .context("fetching ciphertexts")?;

    let items = result
        .as_array()
        .ok_or_else(|| anyhow!("expected array response, got {:?}", result))?;

    if items.is_empty() {
        tracing::info!("inbox empty for UIN {}", my_uin);
        return Ok(());
    }

    for item in items {
        // Convex serializes numbers as JSON floats (e.g. `600000002.0`), so
        // `as_u64` can return None even for whole-number UINs. Fall back to
        // `as_f64` and cast.
        let sender_uin = item["senderUin"]
            .as_u64()
            .or_else(|| item["senderUin"].as_f64().map(|f| f as u64))
            .ok_or_else(|| anyhow!("missing or non-numeric senderUin in {}", item))?;
        let ciphertext_bytes = json_to_bytes(&item["ciphertext"])?;

        let mls_message = MlsMessageIn::tls_deserialize(&mut ciphertext_bytes.as_slice())
            .map_err(|e| anyhow!("deserializing MLS message: {:?}", e))?;

        match mls_message.extract() {
            MlsMessageBodyIn::Welcome(welcome) => {
                let staged = StagedWelcome::new_from_welcome(
                    &provider,
                    &MlsGroupJoinConfig::default(),
                    welcome,
                    None,
                )
                .map_err(|e| anyhow!("staging welcome: {:?}", e))?;
                let group = staged
                    .into_group(&provider)
                    .map_err(|e| anyhow!("joining group: {:?}", e))?;
                let gid = group.group_id().to_vec();
                // Record in sidecar so subsequent `send`/`inbox` can find it.
                provider.record_group_id(&gid)?;
                tracing::info!(
                    "Fetched 1 Welcome from {} — joined group [{}]",
                    sender_uin,
                    hex::encode(&gid[..8.min(gid.len())])
                );
            }
            MlsMessageBodyIn::PrivateMessage(private) => {
                let protocol_message: ProtocolMessage = private.into();
                let group_id_bytes = protocol_message.group_id().to_vec();
                let gid = GroupId::from_slice(&group_id_bytes);
                let mut group = MlsGroup::load(provider.storage(), &gid)
                    .map_err(|e| anyhow!("load group: {:?}", e))?
                    .ok_or_else(|| {
                        anyhow!(
                            "group {} not found locally — missing Welcome?",
                            hex::encode(&group_id_bytes[..8.min(group_id_bytes.len())])
                        )
                    })?;

                let processed = group
                    .process_message(&provider, protocol_message)
                    .map_err(|e| anyhow!("process_message: {:?}", e))?;

                match processed.into_content() {
                    ProcessedMessageContent::ApplicationMessage(app) => {
                        let plaintext =
                            String::from_utf8_lossy(&app.into_bytes()).into_owned();
                        println!("[DECRYPTED] from {}: {}", sender_uin, plaintext);
                    }
                    ProcessedMessageContent::StagedCommitMessage(staged) => {
                        group
                            .merge_staged_commit(&provider, *staged)
                            .map_err(|e| anyhow!("merge_staged_commit: {:?}", e))?;
                        tracing::info!("applied commit from {}", sender_uin);
                    }
                    ProcessedMessageContent::ProposalMessage(_) => {
                        tracing::warn!(
                            "ignoring Proposal from {} (prototype has no proposal flow)",
                            sender_uin
                        );
                    }
                    ProcessedMessageContent::ExternalJoinProposalMessage(_) => {
                        tracing::warn!(
                            "ignoring ExternalJoinProposal from {}",
                            sender_uin
                        );
                    }
                }
            }
            MlsMessageBodyIn::PublicMessage(_) => {
                tracing::warn!("ignoring PublicMessage from {}", sender_uin);
            }
            MlsMessageBodyIn::KeyPackage(_) => {
                tracing::warn!("ignoring orphaned KeyPackage from {}", sender_uin);
            }
            MlsMessageBodyIn::GroupInfo(_) => {
                tracing::warn!("ignoring GroupInfo from {}", sender_uin);
            }
        }
    }

    Ok(())
}
