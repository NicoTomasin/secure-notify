// SPDX-License-Identifier: AGPL-3.0-or-later

//! Embedded, forward-only migrations applied at startup after an automatic backup
//! (docs/engineering/migrations.md#migration-model).

use rusqlite::Connection;

use crate::StorageError;

/// One embedded migration.
#[derive(Debug, Clone, Copy)]
pub struct Migration {
    /// Sequence number; the schema version after this migration ran.
    pub version: u32,
    /// Short description from the file name.
    pub description: &'static str,
    /// SQL to execute.
    pub sql: &'static str,
}

/// The migrations of this binary, in order.
pub const MIGRATIONS: &[Migration] = include!(concat!(env!("OUT_DIR"), "/migrations.rs"));

/// Schema version of a database (`PRAGMA user_version`).
pub(crate) fn schema_version(conn: &Connection) -> Result<u32, StorageError> {
    Ok(conn.query_row("PRAGMA user_version", [], |row| row.get(0))?)
}

/// Whether the database holds anything yet; an empty file needs no backup.
pub(crate) fn is_empty(conn: &Connection) -> Result<bool, StorageError> {
    let count: i64 = conn.query_row("SELECT count(*) FROM sqlite_schema", [], |row| row.get(0))?;
    Ok(count == 0)
}

/// Migrations of `migrations` not yet applied to a database at `version`.
pub(crate) fn pending(
    version: u32,
    migrations: &'static [Migration],
) -> Result<&'static [Migration], StorageError> {
    let latest = migrations.last().map_or(0, |m| m.version);
    if version > latest {
        return Err(StorageError::NewerSchema {
            found: version,
            supported: latest,
        });
    }
    let start = migrations
        .iter()
        .position(|m| m.version > version)
        .unwrap_or(migrations.len());
    Ok(&migrations[start..])
}

/// Applies `pending` migrations in order, each in its own transaction.
pub(crate) fn apply(conn: &mut Connection, pending: &[Migration]) -> Result<(), StorageError> {
    for migration in pending {
        let result = (|| -> rusqlite::Result<()> {
            let tx = conn.transaction()?;
            tx.execute_batch(migration.sql)?;
            tx.pragma_update(None, "user_version", migration.version)?;
            tx.commit()
        })();
        result.map_err(|source| StorageError::Migration {
            version: migration.version,
            description: migration.description,
            source,
        })?;
    }
    Ok(())
}
