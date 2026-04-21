//! `remove` command: eject a peer UIN from the local MLS group via a
//! Remove proposal + Commit, merge the commit, and dispatch the Commit
//! to every remaining member so they can advance their own epoch.
//!
//! 1. Open the SQLite-backed OpenMLS provider.
//! 2. Look up our own UIN record on Convex to get our public signature key,
//!    then load the matching signer from local storage.
//! 3. Load the single local group (prototype one-group-per-device assumption).
//! 4. Scan `group.members()` for the LeafNodeIndex whose credential serializes
//!    to the peer's 8-byte big-endian UIN.
//! 5. Issue `MlsGroup::remove_members` + `merge_pending_commit`.
//! 6. Dispatch the Commit to every remaining member (post-merge the peer is
//!    already gone from `group.members()`, and we skip ourselves).

use anyhow::{anyhow, Context, Result};
use openmls::prelude::{tls_codec::Serialize as _, *};
use openmls_basic_credential::SignatureKeyPair;
use serde_json::json;

use crate::convex_client::{bytes_to_json, json_to_bytes, ConvexClient};
use crate::storage;

const CIPHERSUITE: Ciphersuite =
    Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

pub async fn run(my_uin: u64, peer_uin: u64, db: &str) -> Result<()> {
    let provider = storage::open_provider(db)?;
    let client = ConvexClient::from_env()?;

    // 1. Fetch my UIN record to recover my public signature key (used as
    //    the storage-lookup key for the signer).
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

    // 2. Load the single local group. Same one-group assumption as `send`.
    let group_ids = provider.list_group_ids()?;
    let gid_bytes = group_ids
        .first()
        .ok_or_else(|| anyhow!("no MLS groups in local storage — nothing to remove from"))?
        .clone();
    if group_ids.len() > 1 {
        tracing::warn!(
            "{} groups in local storage; using the first. Prototype limitation.",
            group_ids.len()
        );
    }
    let gid = GroupId::from_slice(&gid_bytes);
    let mut group = MlsGroup::load(provider.storage(), &gid)
        .map_err(|e| anyhow!("load group: {:?}", e))?
        .ok_or_else(|| {
            anyhow!(
                "group {} listed in isy_groups but not loadable from OpenMLS",
                hex::encode(&gid_bytes)
            )
        })?;

    // 3. Find the LeafNodeIndex of the peer we want to remove. Our
    //    credential payload is the 8-byte big-endian UIN.
    let peer_credential_bytes = peer_uin.to_be_bytes();
    let target_leaf = group
        .members()
        .find(|m| m.credential.serialized_content() == peer_credential_bytes.as_slice())
        .ok_or_else(|| anyhow!("UIN {} is not a member of this group", peer_uin))?
        .index;

    // 4. Issue the remove Commit. For a pure Remove with no concurrent Adds,
    //    `welcome` is None. We drop it.
    let (commit, _welcome, _group_info) = group
        .remove_members(&provider, &signer, &[target_leaf])
        .map_err(|e| anyhow!("remove_members failed: {:?}", e))?;
    group
        .merge_pending_commit(&provider)
        .map_err(|e| anyhow!("merge_pending_commit failed: {:?}", e))?;

    let group_id_bytes = group.group_id().to_vec();

    // 5. Dispatch the Commit to every remaining member. Post-merge, the
    //    removed peer is already gone from `group.members()`, so we only
    //    need to skip ourselves.
    let commit_bytes = commit
        .tls_serialize_detached()
        .map_err(|e| anyhow!("serializing commit: {:?}", e))?;

    // Collect recipients first to avoid holding a borrow across await.
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

    let recipient_count = recipients.len();
    for recipient in recipients {
        client
            .mutation(
                "messages:submitCiphertext",
                json!({
                    "recipientUin": recipient,
                    "senderUin": my_uin,
                    "groupId": bytes_to_json(&group_id_bytes),
                    "ciphertext": bytes_to_json(&commit_bytes),
                }),
            )
            .await
            .context("dispatching remove commit")?;
    }

    let gid_preview = hex::encode(&group_id_bytes[..8.min(group_id_bytes.len())]);
    tracing::info!(
        "Removed UIN {} from group [{}], dispatched {} Commit(s)",
        peer_uin,
        gid_preview,
        recipient_count
    );

    Ok(())
}
