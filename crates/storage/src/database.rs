// SPDX-License-Identifier: AGPL-3.0-or-later

//! The database handle: one writer task that serializes every write in a transaction, and a pool
//! of read connections, all running off the async executor
//! (docs/architecture/overview.md#process-model).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use jiff::Timestamp;
use rusqlite::{Connection, OpenFlags, Transaction, TransactionBehavior};
use tokio::sync::{Semaphore, oneshot};

use crate::StorageError;
use crate::migrations::{self, MIGRATIONS, Migration};

/// Number of pre-migration backups kept in the backup directory.
pub const KEPT_MIGRATION_BACKUPS: usize = 3;

/// Prefix of automatic pre-migration backup files.
const MIGRATION_BACKUP_PREFIX: &str = "pre-migration-";

/// How to open the database.
#[derive(Debug, Clone)]
pub struct OpenOptions {
    /// Path of the database file; created when missing.
    pub path: PathBuf,
    /// Directory for automatic pre-migration backups.
    pub backup_dir: PathBuf,
    /// Current time, used to name backups.
    pub now: Timestamp,
    /// Maximum number of read connections.
    pub readers: usize,
}

type Job = Box<dyn FnOnce(&mut Connection) + Send>;

struct Inner {
    path: PathBuf,
    writer: Mutex<Option<mpsc::Sender<Job>>>,
    readers: Mutex<Vec<Connection>>,
    reader_slots: Semaphore,
}

/// Handle to the SQLite database. Cloning is cheap; all clones share the writer and the pool.
#[derive(Clone)]
pub struct Database {
    inner: Arc<Inner>,
}

/// What opening the database did to its schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationReport {
    /// Schema version before opening.
    pub from_version: u32,
    /// Schema version after opening.
    pub to_version: u32,
    /// Backup written before migrating, if any.
    pub backup: Option<PathBuf>,
}

impl Database {
    /// Opens the database, applying pending migrations after a backup. Blocking: call it before
    /// the pipeline starts or from a blocking task.
    pub fn open(options: &OpenOptions) -> Result<(Self, MigrationReport), StorageError> {
        Self::open_with(options, MIGRATIONS)
    }

    fn open_with(
        options: &OpenOptions,
        migrations: &'static [Migration],
    ) -> Result<(Self, MigrationReport), StorageError> {
        let mut conn = Connection::open(&options.path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        configure(&conn)?;

        let from_version = migrations::schema_version(&conn)?;
        let pending = migrations::pending(from_version, migrations)?;
        let mut backup = None;
        if !pending.is_empty() && !migrations::is_empty(&conn)? {
            backup = Some(migration_backup(&conn, options, from_version)?);
        }
        migrations::apply(&mut conn, pending)?;
        let to_version = migrations::schema_version(&conn)?;

        let (sender, receiver) = mpsc::channel::<Job>();
        thread::Builder::new()
            .name("storage-writer".to_string())
            .spawn(move || {
                let mut conn = conn;
                for job in receiver {
                    job(&mut conn);
                }
            })?;

        let database = Self {
            inner: Arc::new(Inner {
                path: options.path.clone(),
                writer: Mutex::new(Some(sender)),
                readers: Mutex::new(Vec::new()),
                reader_slots: Semaphore::new(options.readers.max(1)),
            }),
        };
        let report = MigrationReport {
            from_version,
            to_version,
            backup,
        };
        Ok((database, report))
    }

    /// Runs `f` in a write transaction on the writer task. The transaction commits when `f`
    /// returns `Ok` and rolls back otherwise.
    pub async fn write<R, F>(&self, f: F) -> Result<R, StorageError>
    where
        R: Send + 'static,
        F: FnOnce(&Transaction<'_>) -> Result<R, StorageError> + Send + 'static,
    {
        let (reply, result) = oneshot::channel();
        let job: Job = Box::new(move |conn| {
            let outcome = (|| {
                let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
                let value = f(&tx)?;
                tx.commit()?;
                Ok(value)
            })();
            // The caller may have stopped waiting; the transaction outcome stands either way.
            let _ = reply.send(outcome);
        });
        let sender = lock(&self.inner.writer)
            .clone()
            .ok_or(StorageError::Closed)?;
        sender.send(job).map_err(|_| StorageError::Closed)?;
        result.await.map_err(|_| StorageError::Closed)?
    }

    /// Runs `f` on a read connection from the pool, on a blocking thread.
    pub async fn read<R, F>(&self, f: F) -> Result<R, StorageError>
    where
        R: Send + 'static,
        F: FnOnce(&Connection) -> Result<R, StorageError> + Send + 'static,
    {
        let _permit = self
            .inner
            .reader_slots
            .acquire()
            .await
            .map_err(|_| StorageError::Closed)?;
        let inner = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || {
            let conn = match lock(&inner.readers).pop() {
                Some(conn) => conn,
                None => open_reader(&inner.path)?,
            };
            let result = f(&conn);
            lock(&inner.readers).push(conn);
            result
        })
        .await
        .map_err(|_| StorageError::Closed)?
    }

    /// Writes a consistent copy of the database to `destination`, which must not exist.
    pub async fn backup(&self, destination: &Path) -> Result<(), StorageError> {
        if destination.exists() {
            return Err(StorageError::BackupExists(destination.to_path_buf()));
        }
        let destination = destination.to_path_buf();
        let path = self.inner.path.clone();
        // A dedicated connection: pooled readers are query-only, which `VACUUM INTO` refuses.
        tokio::task::spawn_blocking(move || {
            let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
            configure(&conn)?;
            vacuum_into(&conn, &destination)
        })
        .await
        .map_err(|_| StorageError::Closed)?
    }

    /// Stops accepting writes; pending writes still complete.
    pub fn close(&self) {
        lock(&self.inner.writer).take();
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    // A panic while holding the lock leaves plain data behind, never a broken invariant.
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn configure(conn: &Connection) -> Result<(), StorageError> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    Ok(())
}

fn open_reader(path: &Path) -> Result<Connection, StorageError> {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    configure(&conn)?;
    // Readers never write; writes go through the writer task.
    conn.pragma_update(None, "query_only", "ON")?;
    Ok(conn)
}

fn vacuum_into(conn: &Connection, destination: &Path) -> Result<(), StorageError> {
    let destination = destination
        .to_str()
        .ok_or_else(|| StorageError::BackupPath(destination.to_path_buf()))?;
    conn.execute("VACUUM INTO ?1", [destination])?;
    Ok(())
}

/// Writes the pre-migration backup and prunes older ones.
fn migration_backup(
    conn: &Connection,
    options: &OpenOptions,
    from_version: u32,
) -> Result<PathBuf, StorageError> {
    fs::create_dir_all(&options.backup_dir)?;
    let stamp = options.now.strftime("%Y%m%dT%H%M%SZ");
    let path = options.backup_dir.join(format!(
        "{MIGRATION_BACKUP_PREFIX}{stamp}-v{from_version}.db"
    ));
    if path.exists() {
        return Err(StorageError::BackupExists(path));
    }
    vacuum_into(conn, &path)?;
    prune_migration_backups(&options.backup_dir)?;
    Ok(path)
}

/// Keeps the most recent pre-migration backups; names sort chronologically.
fn prune_migration_backups(dir: &Path) -> Result<(), StorageError> {
    let mut backups: Vec<PathBuf> = fs::read_dir(dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(MIGRATION_BACKUP_PREFIX))
                && p.extension().is_some_and(|e| e == "db")
        })
        .collect();
    backups.sort();
    let excess = backups.len().saturating_sub(KEPT_MIGRATION_BACKUPS);
    for old in &backups[..excess] {
        fs::remove_file(old)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const M1: Migration = Migration {
        version: 1,
        description: "first",
        sql: "CREATE TABLE a (x INTEGER NOT NULL); INSERT INTO a VALUES (1);",
    };
    const M2: Migration = Migration {
        version: 2,
        description: "second",
        sql: "CREATE TABLE b (y INTEGER NOT NULL);",
    };
    const BROKEN: Migration = Migration {
        version: 2,
        description: "broken",
        sql: "CREATE TABLE c (z INTEGER); INSERT INTO missing_table VALUES (1);",
    };

    fn options(dir: &Path, now: &str) -> OpenOptions {
        OpenOptions {
            path: dir.join("db.sqlite3"),
            backup_dir: dir.join("backups"),
            now: now.parse().unwrap(),
            readers: 2,
        }
    }

    fn tables(path: &Path) -> Vec<String> {
        let conn = Connection::open(path).unwrap();
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_schema WHERE type = 'table' ORDER BY name")
            .unwrap();
        stmt.query_map([], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    #[test]
    fn fresh_database_is_migrated_without_backup() {
        let dir = tempfile::tempdir().unwrap();
        let (_, report) =
            Database::open_with(&options(dir.path(), "2026-01-01T00:00:00Z"), &[M1, M2]).unwrap();
        assert_eq!((report.from_version, report.to_version), (0, 2));
        assert_eq!(report.backup, None);
        assert!(!dir.path().join("backups").exists());
    }

    #[test]
    fn existing_database_is_backed_up_before_pending_migrations() {
        let dir = tempfile::tempdir().unwrap();
        let first = options(dir.path(), "2026-01-01T00:00:00Z");
        let (db, _) = Database::open_with(&first, &[M1]).unwrap();
        db.close();
        drop(db);

        let (_, report) =
            Database::open_with(&options(dir.path(), "2026-01-02T00:00:00Z"), &[M1, M2]).unwrap();
        assert_eq!((report.from_version, report.to_version), (1, 2));
        let backup = report.backup.expect("a backup before migrating");
        // The backup holds the schema before the migration.
        assert_eq!(tables(&backup), ["a"]);
        assert_eq!(tables(&first.path), ["a", "b"]);
    }

    #[test]
    fn up_to_date_database_is_not_backed_up_again() {
        let dir = tempfile::tempdir().unwrap();
        let (db, _) =
            Database::open_with(&options(dir.path(), "2026-01-01T00:00:00Z"), &[M1]).unwrap();
        db.close();
        drop(db);
        let (_, report) =
            Database::open_with(&options(dir.path(), "2026-01-02T00:00:00Z"), &[M1]).unwrap();
        assert_eq!(report.backup, None);
    }

    #[test]
    fn database_newer_than_the_binary_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let (db, _) =
            Database::open_with(&options(dir.path(), "2026-01-01T00:00:00Z"), &[M1, M2]).unwrap();
        db.close();
        drop(db);
        let error = Database::open_with(&options(dir.path(), "2026-01-02T00:00:00Z"), &[M1])
            .err()
            .expect("opening must fail");
        assert!(
            matches!(
                error,
                StorageError::NewerSchema {
                    found: 2,
                    supported: 1
                }
            ),
            "{error}"
        );
    }

    #[test]
    fn failed_migration_leaves_the_previous_schema() {
        let dir = tempfile::tempdir().unwrap();
        let opts = options(dir.path(), "2026-01-01T00:00:00Z");
        let error = Database::open_with(&opts, &[M1, BROKEN])
            .err()
            .expect("migration 2 fails");
        assert!(
            matches!(error, StorageError::Migration { version: 2, .. }),
            "{error}"
        );
        let conn = Connection::open(&opts.path).unwrap();
        assert_eq!(migrations::schema_version(&conn).unwrap(), 1);
        assert_eq!(tables(&opts.path), ["a"]);
    }

    #[test]
    fn only_the_most_recent_migration_backups_are_kept() {
        let dir = tempfile::tempdir().unwrap();
        let backups = dir.path().join("backups");
        fs::create_dir_all(&backups).unwrap();
        for day in 1..=4 {
            fs::write(
                backups.join(format!("pre-migration-2026010{day}T000000Z-v1.db")),
                "",
            )
            .unwrap();
        }
        fs::write(backups.join("manual.db"), "").unwrap();
        prune_migration_backups(&backups).unwrap();
        let mut names: Vec<String> = fs::read_dir(&backups)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(
            names,
            [
                "manual.db",
                "pre-migration-20260102T000000Z-v1.db",
                "pre-migration-20260103T000000Z-v1.db",
                "pre-migration-20260104T000000Z-v1.db",
            ]
        );
    }
}
