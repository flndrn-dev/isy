//! `register` command: publishes a UIN + credential + KeyPackages to Convex.
//!
//! 1. Open the per-device SQLite-backed OpenMLS provider.
//! 2. Generate an Ed25519 `SignatureKeyPair` and a `BasicCredential` whose
//!    identity is the 8-byte big-endian UIN.
//! 3. Store the signer in the provider's storage so later commands (add,
//!    send) can reload it.
//! 4. Publish `(credentialBytes, publicSignatureKey)` via `uins:registerUin`.
//! 5. Generate and publish `KEY_PACKAGES_TO_PUBLISH` KeyPackages via
//!    `keyPackages:publishKeyPackage`.

use anyhow::{anyhow, Context, Result};
use openmls::prelude::{tls_codec::Serialize as _, *};
use openmls_basic_credential::SignatureKeyPair;
use serde_json::json;

use crate::convex_client::{bytes_to_json, ConvexClient};
use crate::storage;

const CIPHERSUITE: Ciphersuite =
    Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

const KEY_PACKAGES_TO_PUBLISH: usize = 5;

pub async fn run(uin: u64, db: &str) -> Result<()> {
    let provider = storage::open_provider(db)?;

    // 1. Generate the Ed25519 signing keypair and persist it.
    let signer = SignatureKeyPair::new(CIPHERSUITE.signature_algorithm())
        .map_err(|e| anyhow!("SignatureKeyPair::new failed: {:?}", e))?;
    signer
        .store(provider.storage())
        .map_err(|e| anyhow!("signer.store failed: {:?}", e))?;

    // 2. Build the BasicCredential (identity = 8-byte big-endian UIN).
    //    `BasicCredential` itself does not implement `TlsSerialize`; the
    //    generic `Credential` wrapper (the `.into()` target) does. Serialize
    //    from the wrapper for transport.
    let basic_credential = BasicCredential::new(uin.to_be_bytes().to_vec());
    let credential: Credential = basic_credential.into();
    let credential_with_key = CredentialWithKey {
        credential: credential.clone(),
        signature_key: signer.to_public_vec().into(),
    };

    // 3. Serialize the credential for transport.
    let credential_bytes = credential
        .tls_serialize_detached()
        .map_err(|e| anyhow!("credential.tls_serialize_detached failed: {:?}", e))?;
    let public_signature_key = signer.to_public_vec();

    // 4. Register the UIN.
    let client = ConvexClient::from_env()?;
    client
        .mutation(
            "uins:registerUin",
            json!({
                "uin": uin,
                "credentialBytes": bytes_to_json(&credential_bytes),
                "publicSignatureKey": bytes_to_json(&public_signature_key),
            }),
        )
        .await
        .context("registering uin")?;

    tracing::info!("Registered UIN {}", uin);

    // 5. Publish KeyPackages.
    for i in 0..KEY_PACKAGES_TO_PUBLISH {
        let bundle = KeyPackage::builder()
            .build(CIPHERSUITE, &provider, &signer, credential_with_key.clone())
            .map_err(|e| anyhow!("KeyPackage::builder().build failed: {:?}", e))?;
        let kp_bytes = bundle
            .key_package()
            .tls_serialize_detached()
            .map_err(|e| anyhow!("key_package.tls_serialize_detached failed: {:?}", e))?;
        client
            .mutation(
                "keyPackages:publishKeyPackage",
                json!({
                    "uin": uin,
                    "keyPackageBytes": bytes_to_json(&kp_bytes),
                }),
            )
            .await
            .context("publishing key package")?;
        tracing::debug!(
            "Published KeyPackage {} of {}",
            i + 1,
            KEY_PACKAGES_TO_PUBLISH
        );
    }

    tracing::info!(
        "Registered UIN {}, published {} KeyPackages",
        uin,
        KEY_PACKAGES_TO_PUBLISH
    );

    Ok(())
}
