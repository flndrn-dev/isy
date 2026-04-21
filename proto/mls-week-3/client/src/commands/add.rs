//! `add` command: add a peer UIN to an MLS group, reusing an existing
//! local group if one is present.
//!
//! 1. Open the SQLite-backed OpenMLS provider.
//! 2. Look up our own UIN record on Convex to get our public signature
//!    key, then load the matching signer from local storage.
//! 3. Fetch one of the peer's KeyPackages from Convex (consumed on the
//!    server).
//! 4. If the device already has an MLS group on file, load it; otherwise
//!    create a new one with `use_ratchet_tree_extension(true)` so the
//!    Welcome will carry the tree in-band (Task 7 discovery).
//! 5. Add the peer with `MlsGroup::add_members` + `merge_pending_commit`.
//! 6. Dispatch the Welcome to the peer's inbox via `messages:submitCiphertext`.
//!    If we joined an existing group, also dispatch the Commit to every
//!    other existing member.
//!
//! OpenMLS 0.8.1 does not expose a group-enumeration API (only
//! `MlsGroup::load(storage, group_id)`). Group reuse is therefore driven
//! by a small sidecar table `isy_groups` (see `storage::IsyProvider`).

use anyhow::{anyhow, Context, Result};
use openmls::prelude::{tls_codec::{Deserialize as _, Serialize as _}, *};
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

    // 2. Load my signer. In openmls_basic_credential 0.5 the loader is
    //    `SignatureKeyPair::read(store, public_key, signature_scheme) -> Option<Self>`
    //    (not `load`); see openmls_basic_credential-0.5.0/src/lib.rs:128.
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

    let my_credential = Credential::from(BasicCredential::new(my_uin.to_be_bytes().to_vec()));
    let my_credential_with_key = CredentialWithKey {
        credential: my_credential,
        signature_key: signer.to_public_vec().into(),
    };

    // 3. Fetch a peer KeyPackage. This is a mutation because the server
    //    atomically marks the row consumed.
    let kp_value = client
        .mutation("keyPackages:fetchKeyPackage", json!({"uin": peer_uin}))
        .await
        .context("fetching peer KeyPackage")?;
    let kp_bytes = json_to_bytes(&kp_value).context("parsing peer KeyPackage bytes")?;
    let peer_key_package_in = KeyPackageIn::tls_deserialize(&mut kp_bytes.as_slice())
        .map_err(|e| anyhow!("deserializing peer KeyPackage: {:?}", e))?;
    let peer_key_package = peer_key_package_in
        .validate(provider.crypto(), ProtocolVersion::Mls10)
        .map_err(|e| anyhow!("validating peer KeyPackage: {:?}", e))?;

    // 4. Decide between reuse and create. See module-level comment for why
    //    we maintain the sidecar table ourselves.
    let existing_group_ids = provider.list_group_ids()?;

    let create_config = MlsGroupCreateConfig::builder()
        .use_ratchet_tree_extension(true)
        .build();

    let (mut group, is_new_group) = if let Some(gid_bytes) = existing_group_ids.first() {
        let gid = GroupId::from_slice(gid_bytes);
        let g = MlsGroup::load(provider.storage(), &gid)
            .map_err(|e| anyhow!("loading group: {:?}", e))?
            .ok_or_else(|| {
                anyhow!(
                    "group {} listed in isy_groups but not loadable from OpenMLS",
                    hex::encode(gid_bytes)
                )
            })?;
        (g, false)
    } else {
        let g = MlsGroup::new(
            &provider,
            &signer,
            &create_config,
            my_credential_with_key.clone(),
        )
        .map_err(|e| anyhow!("creating new group: {:?}", e))?;
        (g, true)
    };

    // 5. Add the peer and merge the resulting Commit so the group's
    //    local state advances before we serialize anything.
    let (commit, welcome, _group_info) = group
        .add_members(
            &provider,
            &signer,
            core::slice::from_ref(&peer_key_package),
        )
        .map_err(|e| anyhow!("add_members failed: {:?}", e))?;
    group
        .merge_pending_commit(&provider)
        .map_err(|e| anyhow!("merge_pending_commit failed: {:?}", e))?;

    let group_id = group.group_id().to_vec();

    // Record the group ID in our sidecar table if it's freshly created.
    if is_new_group {
        provider.record_group_id(&group_id)?;
    }

    // 6. Ship the Welcome to the new peer.
    let welcome_bytes = welcome
        .tls_serialize_detached()
        .map_err(|e| anyhow!("serializing welcome: {:?}", e))?;
    client
        .mutation(
            "messages:submitCiphertext",
            json!({
                "recipientUin": peer_uin,
                "senderUin": my_uin,
                "groupId": bytes_to_json(&group_id),
                "ciphertext": bytes_to_json(&welcome_bytes),
            }),
        )
        .await
        .context("submitting welcome to peer inbox")?;

    // 7. If we're adding to an already-populated group, existing
    //    members (everyone other than us and the freshly-added peer)
    //    need the Commit so they can advance their own copy of the
    //    group's epoch.
    let mut commit_dispatched = 0usize;
    if !is_new_group {
        let commit_bytes = commit
            .tls_serialize_detached()
            .map_err(|e| anyhow!("serializing commit: {:?}", e))?;
        // Collect members first so we don't borrow `group` across the await.
        let member_uins: Vec<u64> = group
            .members()
            .filter_map(|m| {
                let cred_bytes = m.credential.serialized_content();
                if cred_bytes.len() != 8 {
                    return None; // non-UIN credential — ignore
                }
                let mut arr = [0u8; 8];
                arr.copy_from_slice(cred_bytes);
                let uin = u64::from_be_bytes(arr);
                if uin == my_uin || uin == peer_uin {
                    None
                } else {
                    Some(uin)
                }
            })
            .collect();
        for member_uin in member_uins {
            client
                .mutation(
                    "messages:submitCiphertext",
                    json!({
                        "recipientUin": member_uin,
                        "senderUin": my_uin,
                        "groupId": bytes_to_json(&group_id),
                        "ciphertext": bytes_to_json(&commit_bytes),
                    }),
                )
                .await
                .context("dispatching commit to existing member")?;
            commit_dispatched += 1;
        }
    }

    let gid_preview = hex::encode(&group_id[..8.min(group_id.len())]);
    tracing::info!(
        "{} group [{}], added UIN {}, dispatched 1 Welcome and {} Commit(s)",
        if is_new_group {
            "Created"
        } else {
            "Joined existing"
        },
        gid_preview,
        peer_uin,
        commit_dispatched
    );

    Ok(())
}
