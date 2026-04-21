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
///
/// Also remembers the db path so higher-level code can open short-lived
/// sidecar connections (see `list_group_ids`), since OpenMLS 0.8.1 does
/// not expose a group-enumeration API.
pub struct IsyProvider {
    crypto: RustCrypto,
    storage: SqliteStorageProvider<JsonCodec, Connection>,
    db_path: String,
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

impl IsyProvider {
    /// Enumerate all MLS group IDs this device knows about.
    ///
    /// OpenMLS 0.8.1 has no public API for listing group IDs (see
    /// openmls-0.8.1/src/group/mls_group/mod.rs — only `load(storage,
    /// group_id)` exists, not `load_all`). We maintain a sidecar table
    /// `isy_groups(group_id BLOB PRIMARY KEY)` alongside the OpenMLS
    /// tables in the same SQLite file and query it directly via a
    /// short-lived second connection.
    pub fn list_group_ids(&self) -> Result<Vec<Vec<u8>>> {
        let conn = Connection::open(&self.db_path)
            .with_context(|| format!("sidecar open {}", self.db_path))?;
        let mut stmt = conn
            .prepare("SELECT group_id FROM isy_groups")
            .context("preparing isy_groups select")?;
        let rows = stmt
            .query_map([], |row| row.get::<_, Vec<u8>>(0))
            .context("running isy_groups select")?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.context("reading isy_groups row")?);
        }
        Ok(out)
    }

    /// Record that this device has joined / created the group with the
    /// given ID. Idempotent via `INSERT OR IGNORE`.
    pub fn record_group_id(&self, group_id: &[u8]) -> Result<()> {
        let conn = Connection::open(&self.db_path)
            .with_context(|| format!("sidecar open {}", self.db_path))?;
        conn.execute(
            "INSERT OR IGNORE INTO isy_groups (group_id) VALUES (?1)",
            rusqlite::params![group_id],
        )
        .context("inserting into isy_groups")?;
        Ok(())
    }
}

/// Open or create a SQLite-backed OpenMLS provider at the given path.
///
/// Runs the storage provider's schema migrations on every open — refinery
/// tracks applied migrations in its own table, so calling this on an
/// already-initialized database is a no-op. Also ensures the sidecar
/// `isy_groups` table exists (see `IsyProvider::list_group_ids`).
pub fn open_provider(db_path: &str) -> Result<IsyProvider> {
    let connection = Connection::open(db_path)
        .with_context(|| format!("failed to open sqlite database at {}", db_path))?;
    let mut storage = SqliteStorageProvider::<JsonCodec, Connection>::new(connection);
    storage
        .run_migrations()
        .context("failed to run openmls_sqlite_storage migrations")?;

    // Sidecar table for group enumeration. Opened on a second short-lived
    // connection so we don't fight the storage provider for ownership of
    // its `Connection`. SQLite handles intra-process multi-connection
    // access cleanly for our serial CLI workload.
    {
        let conn = Connection::open(db_path)
            .with_context(|| format!("sidecar open {}", db_path))?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS isy_groups (group_id BLOB PRIMARY KEY)",
            [],
        )
        .context("creating isy_groups table")?;
    }

    Ok(IsyProvider {
        crypto: RustCrypto::default(),
        storage,
        db_path: db_path.to_string(),
    })
}
