// SPDX-License-Identifier: AGPL-3.0-or-later

//! SQLite persistence for secure-notify, with the embedded migrations.
//!
//! SQLite runs in WAL mode with a single writer task and a pool of read connections
//! (docs/architecture/overview.md#process-model). Migrations in `migrations/` are embedded,
//! forward-only, and applied when the database is opened, after an automatic backup
//! (docs/engineering/migrations.md).

mod database;
mod instance;
mod inventory;
mod migrations;
mod source_state;
mod subscribers;
mod time;
mod watches;

use std::path::PathBuf;

pub use database::{Database, KEPT_MIGRATION_BACKUPS, MigrationReport, OpenOptions};
pub use migrations::{MIGRATIONS, Migration};

/// Errors of the storage layer.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// SQLite reported an error.
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// A migration failed; the database keeps the schema of the previous migration.
    #[error("migration {version:04} ({description}) failed: {source}")]
    Migration {
        /// Sequence number of the failed migration.
        version: u32,
        /// Its description.
        description: &'static str,
        /// The SQLite error.
        source: rusqlite::Error,
    },
    /// The database was migrated by a newer binary.
    #[error(
        "the database has schema version {found}, but this binary supports up to {supported}; \
         use a newer binary or restore a backup"
    )]
    NewerSchema {
        /// Version found in the database.
        found: u32,
        /// Latest version this binary knows.
        supported: u32,
    },
    /// The backup destination already exists.
    #[error("backup destination {0} already exists")]
    BackupExists(PathBuf),
    /// The backup destination path is not valid UTF-8.
    #[error("backup destination {0} is not a valid UTF-8 path")]
    BackupPath(PathBuf),
    /// A file system operation failed.
    #[error("file system error: {0}")]
    Io(#[from] std::io::Error),
    /// The database handle was closed.
    #[error("the database is closed")]
    Closed,
    /// Stored data could not be decoded.
    #[error("invalid stored data: {0}")]
    Corrupt(String),
}
