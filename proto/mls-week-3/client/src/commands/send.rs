//! `send` command: encrypt a plaintext for the single local MLS group and
//! push the resulting ciphertext to every group member's Convex inbox
//! (except ourselves).
//!
//! 1. Open the SQLite-backed OpenMLS provider.
//! 2. Look up our own UIN record on Convex to get our public signature key,
//!    then load the matching signer from local storage.
//! 3. Find the single group in the sidecar table (prototype assumption:
//!    exactly one group per device; warn if more).
//! 4. Encrypt with `MlsGroup::create_message`, serialize via
//!    `tls_serialize_detached`, and fan out through `messages:submitCiphertext`
//!    to every other group member. The `--peer-uin` CLI flag is used only
//!    as a "which group" hint for the one-group-per-device prototype and is
//!    verified to be a member; the ciphertext itself is an MLS group message
//!    that all members must receive in order to decrypt.

use anyhow::{anyhow, Context, Result};
use openmls::prelude::{tls_codec::Serialize as _, *};
use openmls_basic_credential::SignatureKeyPair;
use serde_json::json;

use crate::convex_client::{bytes_to_json, json_to_bytes, ConvexClient};
use crate::storage;

const CIPHERSUITE: Ciphersuite =
    Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

pub async fn run(my_uin: u64, peer_uin: u64, message: &str, db: &str) -> Result<()> {
    let provider = storage::open_provider(db)?;
    let client = ConvexClient::from_env()?;

    // Load my signer via Convex pubkey lookup (same pattern as `add`).
    let my_record = client
        .query("uins:lookupUin", json!({"uin": my_uin}))
        .await
        .context("looking up my UIN")?;
    if my_record.is_null() {
        return Err(anyhow!(
            "UIN {} not registered on server — run `register` first",
            my_uin
        ));
    }
    let my_pubkey = json_to_bytes(&my_record["publicSignatureKey"])
        .context("parsing publicSignatureKey from my record")?;
    let signer = SignatureKeyPair::read(
        provider.storage(),
        &my_pubkey,
        CIPHERSUITE.signature_algorithm(),
    )
    .ok_or_else(|| {
        anyhow!(
            "signer for UIN {} not found in {} — did you run `register` on this device?",
            my_uin,
            db
        )
    })?;

    // Find the single group in local storage.
    let group_ids = provider.list_group_ids()?;
    let gid_bytes = group_ids
        .first()
        .ok_or_else(|| anyhow!("no MLS groups in local storage — run `add` first"))?;
    if group_ids.len() > 1 {
        tracing::warn!(
            "{} groups in local storage; using the first. Prototype limitation.",
            group_ids.len()
        );
    }
    let gid = GroupId::from_slice(gid_bytes);
    let mut group = MlsGroup::load(provider.storage(), &gid)
        .map_err(|e| anyhow!("load group: {:?}", e))?
        .ok_or_else(|| {
            anyhow!(
                "group {} listed in isy_groups but not loadable from OpenMLS",
                hex::encode(gid_bytes)
            )
        })?;

    // Encrypt.
    let message_out = group
        .create_message(&provider, &signer, message.as_bytes())
        .map_err(|e| anyhow!("create_message: {:?}", e))?;
    let ciphertext = message_out
        .tls_serialize_detached()
        .map_err(|e| anyhow!("serializing message: {:?}", e))?;

    // Collect all other group members — MLS application messages are
    // encrypted to the group, so every member except us must receive the
    // bytes in order to decrypt. `peer_uin` serves only as a "which group"
    // hint for the one-group-per-device prototype.
    let group_id_vec = group.group_id().to_vec();
    let recipients: Vec<u64> = group
        .members()
        .filter_map(|m| {
            let cred = m.credential.serialized_content();
            if cred.len() != 8 {
                return None;
            }
            let mut arr = [0u8; 8];
            arr.copy_from_slice(cred);
            let uin = u64::from_be_bytes(arr);
            if uin == my_uin {
                None
            } else {
                Some(uin)
            }
        })
        .collect();

    if !recipients.contains(&peer_uin) {
        return Err(anyhow!(
            "UIN {} is not a member of this group; cannot send",
            peer_uin
        ));
    }

    // Submit to Convex once per recipient.
    let recipient_count = recipients.len();
    for recipient in recipients {
        client
            .mutation(
                "messages:submitCiphertext",
                json!({
                    "recipientUin": recipient,
                    "senderUin": my_uin,
                    "groupId": bytes_to_json(&group_id_vec),
                    "ciphertext": bytes_to_json(&ciphertext),
                }),
            )
            .await
            .context("submitting ciphertext")?;
    }

    let gid_preview = hex::encode(&group_id_vec[..8.min(group_id_vec.len())]);
    tracing::info!(
        "Encrypted and submitted {} ciphertext for group [{}]",
        recipient_count,
        gid_preview
    );

    Ok(())
}
