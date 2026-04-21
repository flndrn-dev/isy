//! SQLite-backed OpenMLS provider.
//!
//! `openmls_rust_crypto::OpenMlsRustCrypto` in 0.5.1 is hard-wired to use
//! `MemoryStorage` and exposes no constructor for swapping the storage
//! backend. To get persistence across CLI invocations we build our own
//! lightweight provider struct that pairs `RustCrypto` (the crypto +
//! randomness provider exported publicly by `openmls_rust_crypto`) with a
//! `SqliteStorageProvider` from `openmls_sqlite_storage`.
//!
//! Implementing `OpenMlsProvider` is trivial — it is purely a getter trait
//! over three sub-providers (see openmls_traits-0.5.0/src/traits.rs line
//! 29). Later tasks can pass `&IsyProvider` anywhere the book-code example
//! passes `&OpenMlsRustCrypto`.

use anyhow::{Context, Result};
use openmls::prelude::OpenMlsProvider;
use openmls_rust_crypto::RustCrypto;
use openmls_sqlite_storage::{Codec, Connection, SqliteStorageProvider};
use serde::Serialize;

/// JSON codec for the SQLite storage provider. The codec crate does not
/// ship a default impl; the pattern here mirrors
/// openmls_sqlite_storage-0.2.0/tests/proposals.rs line 12.
#[derive(Default)]
pub struct JsonCodec;

impl Codec for JsonCodec {
    type Error = serde_json::Error;

    fn to_vec<T: Serialize>(value: &T) -> Result<Vec<u8>, Self::Error> {
        serde_json::to_vec(value)
    }

    fn from_slice<T: serde::de::DeserializeOwned>(slice: &[u8]) -> Result<T, Self::Error> {
        serde_json::from_slice(slice)
    }
}

/// SQLite-backed OpenMLS provider. Holds a `RustCrypto` for crypto +
/// randomness and a `SqliteStorageProvider` for persistent MLS state.
pub struct IsyProvider {
    crypto: RustCrypto,
    storage: SqliteStorageProvider<JsonCodec, Connection>,
}

impl OpenMlsProvider for IsyProvider {
    type CryptoProvider = RustCrypto;
    type RandProvider = RustCrypto;
    type StorageProvider = SqliteStorageProvider<JsonCodec, Connection>;

    fn storage(&self) -> &Self::StorageProvider {
        &self.storage
    }

    fn crypto(&self) -> &Self::CryptoProvider {
        &self.crypto
    }

    fn rand(&self) -> &Self::RandProvider {
        &self.crypto
    }
}

/// Open or create a SQLite-backed OpenMLS provider at the given path.
///
/// Runs the storage provider's schema migrations on every open — refinery
/// tracks applied migrations in its own table, so calling this on an
/// already-initialized database is a no-op.
pub fn open_provider(db_path: &str) -> Result<IsyProvider> {
    let connection = Connection::open(db_path)
        .with_context(|| format!("failed to open sqlite database at {}", db_path))?;
    let mut storage = SqliteStorageProvider::<JsonCodec, Connection>::new(connection);
    storage
        .run_migrations()
        .context("failed to run openmls_sqlite_storage migrations")?;
    Ok(IsyProvider {
        crypto: RustCrypto::default(),
        storage,
    })
}
