//! In-memory two-party OpenMLS proof-of-concept.
//!
//! Validates the OpenMLS 0.8 + companion-crate 0.5.x API shape end-to-end:
//!  1. Alice and Bob each have their own provider + signer + credential.
//!  2. Bob publishes a KeyPackage.
//!  3. Alice creates a group and adds Bob.
//!  4. Bob joins via the Welcome.
//!  5. Alice encrypts "hello bob".
//!  6. Bob decrypts; plaintext must match.
//!
//! All in-process. No Convex, no networking, no SQLite. Purely validates the
//! crypto-layer APIs before Tasks 10-13 integrate against a real transport.

use anyhow::{anyhow, Result};
use openmls::prelude::{tls_codec::*, *};
use openmls_basic_credential::SignatureKeyPair;
use openmls_rust_crypto::OpenMlsRustCrypto;

const CIPHERSUITE: Ciphersuite =
    Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

/// Build a `(CredentialWithKey, SignatureKeyPair)` pair, storing the signer
/// in the provider's storage. Mirrors `generate_credential` in the openmls
/// book tests (openmls-0.8.1/tests/book_code.rs).
fn generate_credential(
    identity: Vec<u8>,
    provider: &OpenMlsRustCrypto,
) -> Result<(CredentialWithKey, SignatureKeyPair)> {
    let credential = BasicCredential::new(identity);
    let signature_keys = SignatureKeyPair::new(CIPHERSUITE.signature_algorithm())
        .map_err(|e| anyhow!("SignatureKeyPair::new failed: {:?}", e))?;
    signature_keys
        .store(provider.storage())
        .map_err(|e| anyhow!("signature_keys.store failed: {:?}", e))?;

    Ok((
        CredentialWithKey {
            credential: credential.into(),
            signature_key: signature_keys.to_public_vec().into(),
        },
        signature_keys,
    ))
}

pub fn run_poc() -> Result<()> {
    tracing::info!("mls_poc: start");

    // 1. Identities.
    let alice_provider = OpenMlsRustCrypto::default();
    let bob_provider = OpenMlsRustCrypto::default();

    let (alice_credential, alice_signer) =
        generate_credential(b"alice".to_vec(), &alice_provider)?;
    let (bob_credential, bob_signer) =
        generate_credential(b"bob".to_vec(), &bob_provider)?;

    // 2. Bob publishes a KeyPackage.
    let bob_key_package_bundle = KeyPackage::builder()
        .build(
            CIPHERSUITE,
            &bob_provider,
            &bob_signer,
            bob_credential.clone(),
        )
        .map_err(|e| anyhow!("KeyPackage::builder().build failed: {:?}", e))?;
    tracing::info!("mls_poc: bob published KeyPackage");

    // 3. Alice creates a group.
    // Enable the ratchet-tree extension so the Welcome carries the tree
    // in-band. Without it, StagedWelcome::new_from_welcome fails with
    // `MissingRatchetTree` because Bob has no way to learn the tree.
    let create_config = MlsGroupCreateConfig::builder()
        .use_ratchet_tree_extension(true)
        .build();
    let mut alice_group = MlsGroup::new(
        &alice_provider,
        &alice_signer,
        &create_config,
        alice_credential.clone(),
    )
    .map_err(|e| anyhow!("MlsGroup::new failed: {:?}", e))?;
    tracing::info!("mls_poc: alice created group");

    // 4. Alice adds Bob. add_members returns (commit_msg, welcome, group_info).
    let (_commit, welcome_out, _group_info) = alice_group
        .add_members(
            &alice_provider,
            &alice_signer,
            core::slice::from_ref(bob_key_package_bundle.key_package()),
        )
        .map_err(|e| anyhow!("add_members failed: {:?}", e))?;
    alice_group
        .merge_pending_commit(&alice_provider)
        .map_err(|e| anyhow!("merge_pending_commit failed: {:?}", e))?;
    tracing::info!("mls_poc: alice added bob, members={}", alice_group.members().count());

    // 5. Bob joins via the Welcome.
    // `MlsMessageIn::into_welcome` and `MlsMessageOut::into_welcome` are both
    // gated behind the `test-utils` feature in openmls 0.8.1. For production
    // code we serialize the MlsMessageOut to bytes, deserialize into
    // MlsMessageIn, then match on the public `extract()` → `MlsMessageBodyIn`.
    // (Source: openmls-0.8.1/src/framing/message_{in,out}.rs lines 149 / 167.)
    let welcome_bytes = welcome_out
        .tls_serialize_detached()
        .map_err(|e| anyhow!("welcome_out.tls_serialize_detached failed: {:?}", e))?;
    let welcome_in = MlsMessageIn::tls_deserialize_exact(&welcome_bytes)
        .map_err(|e| anyhow!("deserialize welcome: {:?}", e))?;
    let welcome = match welcome_in.extract() {
        MlsMessageBodyIn::Welcome(w) => w,
        _ => return Err(anyhow!("expected Welcome message body")),
    };

    let staged_join = StagedWelcome::new_from_welcome(
        &bob_provider,
        &MlsGroupJoinConfig::default(),
        welcome,
        None,
    )
    .map_err(|e| anyhow!("StagedWelcome::new_from_welcome failed: {:?}", e))?;
    let mut bob_group = staged_join
        .into_group(&bob_provider)
        .map_err(|e| anyhow!("staged_join.into_group failed: {:?}", e))?;
    tracing::info!("mls_poc: bob joined group, members={}", bob_group.members().count());

    // Sanity: both groups should share the same epoch authenticator.
    if alice_group.epoch_authenticator().as_slice()
        != bob_group.epoch_authenticator().as_slice()
    {
        return Err(anyhow!("epoch authenticator mismatch between alice and bob"));
    }

    // 6. Alice encrypts "hello bob".
    let plaintext_in = b"hello bob";
    let mls_message_out = alice_group
        .create_message(&alice_provider, &alice_signer, plaintext_in)
        .map_err(|e| anyhow!("create_message failed: {:?}", e))?;
    tracing::info!("mls_poc: alice created ciphertext");

    // Serialize -> deserialize to exercise the wire format (closer to real use).
    let wire = mls_message_out
        .to_bytes()
        .map_err(|e| anyhow!("mls_message_out.to_bytes failed: {:?}", e))?;
    let mls_message_in = MlsMessageIn::tls_deserialize_exact(wire)
        .map_err(|e| anyhow!("MlsMessageIn::tls_deserialize_exact failed: {:?}", e))?;

    // 7. Bob processes and decrypts.
    let protocol_message: ProtocolMessage = mls_message_in
        .try_into_protocol_message()
        .map_err(|e| anyhow!("try_into_protocol_message failed: {:?}", e))?;
    let processed = bob_group
        .process_message(&bob_provider, protocol_message)
        .map_err(|e| anyhow!("process_message failed: {:?}", e))?;

    let plaintext_out = match processed.into_content() {
        ProcessedMessageContent::ApplicationMessage(app) => {
            let bytes = app.into_bytes();
            String::from_utf8(bytes).map_err(|e| anyhow!("utf8 decode: {}", e))?
        }
        _ => return Err(anyhow!("expected ApplicationMessage")),
    };

    tracing::info!("bob decrypted: {}", plaintext_out);

    if plaintext_out != "hello bob" {
        return Err(anyhow!(
            "plaintext mismatch: got {:?}, expected {:?}",
            plaintext_out,
            "hello bob"
        ));
    }

    tracing::info!("mls_poc: ok");
    Ok(())
}
