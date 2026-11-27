// SPDX-License-Identifier: AGPL-3.0-or-later

//! Storage layer tests on real SQLite with the real migrations applied
//! (docs/engineering/testing.md#test-layers).

// Test helpers outside `#[test]` functions may unwrap too.
#![allow(clippy::unwrap_used)]

use std::path::Path;

use rusqlite::{Connection, params};
use secure_notify_storage::{Database, MIGRATIONS, OpenOptions, StorageError};

fn open(dir: &Path) -> Database {
    let options = OpenOptions {
        path: dir.join("secure-notify.db"),
        backup_dir: dir.join("backups"),
        now: "2026-11-27T12:00:00Z".parse().unwrap(),
        readers: 2,
    };
    Database::open(&options).unwrap().0
}

/// The schema produced by every migration, one statement per entry, in a stable order.
fn schema(conn: &Connection) -> String {
    let mut stmt = conn
        .prepare(
            "SELECT sql FROM sqlite_schema WHERE sql IS NOT NULL \
             ORDER BY tbl_name, type DESC, name",
        )
        .unwrap();
    let statements: Vec<String> = stmt
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    statements.join(";\n\n") + ";\n"
}

#[test]
fn schema_matches_the_reviewed_snapshot() {
    // docs/engineering/migrations.md#testing-migrations: every migration applied to an empty
    // database, compared with a committed, reviewed snapshot.
    let dir = tempfile::tempdir().unwrap();
    let db = open(dir.path());
    db.close();
    let conn = Connection::open(dir.path().join("secure-notify.db")).unwrap();
    let version: u32 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, MIGRATIONS.last().unwrap().version);
    insta::assert_snapshot!("schema", schema(&conn));
}

/// An insert expected to violate a constraint.
type Insert<'a> = Box<dyn Fn() -> rusqlite::Result<usize> + 'a>;

fn insert_person(conn: &Connection, role: &str) -> i64 {
    conn.execute(
        "INSERT INTO subscribers (kind, role, created_at) VALUES ('person', ?1, '2026-11-27T12:00:00Z')",
        [role],
    )
    .unwrap();
    conn.last_insert_rowid()
}

#[test]
fn schema_constraints_reject_invalid_rows() {
    let dir = tempfile::tempdir().unwrap();
    let db = open(dir.path());
    db.close();
    let conn = Connection::open(dir.path().join("secure-notify.db")).unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    let person = insert_person(&conn, "admin");
    conn.execute(
        "INSERT INTO endpoints (subscriber_id, platform, address, kind, created_at) \
         VALUES (?1, 'telegram', '100', 'private', '2026-11-27T12:00:00Z')",
        [person],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO stories (kind, title, title_priority, severity_level, first_seen_at, updated_at, backfill) \
         VALUES ('vulnerability', 't', 0, 'unknown', '2026-11-27T12:00:00Z', '2026-11-27T12:00:00Z', 0)",
        [],
    )
    .unwrap();
    let story = conn.last_insert_rowid();
    let event = "INSERT INTO events (source, source_item_id, story_id, content_hash, fetched_at, updated_at, backfill) \
                 VALUES ('osv', 'GHSA-35jh-r3h4-6jhm', ?1, 'h', '2026-11-27T12:00:00Z', '2026-11-27T12:00:00Z', 0)";
    conn.execute(event, [story]).unwrap();

    let cases: [(&str, Insert); 5] = [
        (
            // overview.md: unique (source, source_item_id), so re-fetching never duplicates.
            "same source item twice",
            Box::new(|| conn.execute(event, [story])),
        ),
        (
            // subscribers.md: instance roles belong to persons only.
            "person without a role",
            Box::new(|| {
                conn.execute(
                    "INSERT INTO subscribers (kind, created_at) VALUES ('person', '2026-11-27T12:00:00Z')",
                    [],
                )
            }),
        ),
        (
            "team with a role",
            Box::new(|| {
                conn.execute(
                    "INSERT INTO subscribers (kind, role, created_at) VALUES ('team', 'admin', '2026-11-27T12:00:00Z')",
                    [],
                )
            }),
        ),
        (
            // subscribers.md: an endpoint is linked to exactly one subscriber.
            "endpoint linked twice",
            Box::new(|| {
                conn.execute(
                    "INSERT INTO endpoints (subscriber_id, platform, address, kind, created_at) \
                     VALUES (?1, 'telegram', '100', 'private', '2026-11-27T12:00:00Z')",
                    [person],
                )
            }),
        ),
        (
            "same inventory entry twice",
            Box::new(|| {
                conn.execute(
                    "INSERT INTO inventory_entries (subscriber_id, component, version, last_seen_at) \
                     VALUES (?1, 'pkg:npm/lodash', '4.17.20', 'x'), (?1, 'pkg:npm/lodash', '4.17.20', 'y')",
                    [person],
                )
            }),
        ),
    ];
    for (case, insert) in cases {
        assert!(insert().is_err(), "case: {case}");
    }
}

#[test]
fn removing_a_subscriber_deletes_the_data_it_owns() {
    // subscribers.md: a subscriber owns watches, inventory, rules, and mutes; an admin removing it
    // must not leave them behind.
    let dir = tempfile::tempdir().unwrap();
    let db = open(dir.path());
    db.close();
    let conn = Connection::open(dir.path().join("secure-notify.db")).unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    let person = insert_person(&conn, "member");
    let t = "2026-11-27T12:00:00Z";
    conn.execute_batch(&format!(
        "INSERT INTO endpoints (subscriber_id, platform, address, kind, created_at) VALUES ({person}, 'telegram', '7', 'private', '{t}');
         INSERT INTO watches (subscriber_id, entity_id, origin, created_at) VALUES ({person}, 'tech:node', 'manual', '{t}');
         INSERT INTO inventory_entries (subscriber_id, component, version, last_seen_at) VALUES ({person}, 'pkg:npm/lodash', '4.17.20', '{t}');
         INSERT INTO rules (subscriber_id, position, match, action) VALUES ({person}, 1, '{{}}', 'notify');
         INSERT INTO mutes (subscriber_id, scope, value, created_at) VALUES ({person}, 'entity', 'tech:node', '{t}');"
    ))
    .unwrap();
    conn.execute("DELETE FROM subscribers WHERE id = ?1", [person])
        .unwrap();
    for table in [
        "endpoints",
        "watches",
        "inventory_entries",
        "rules",
        "mutes",
    ] {
        let count: i64 = conn
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0, "rows left in {table}");
    }
}

#[tokio::test]
async fn failed_write_is_rolled_back() {
    let dir = tempfile::tempdir().unwrap();
    let db = open(dir.path());
    let result: Result<(), StorageError> = db
        .write(|tx| {
            tx.execute(
                "INSERT INTO instance_state (key, value) VALUES ('k', 'v')",
                [],
            )?;
            Err(StorageError::Corrupt("abort".to_string()))
        })
        .await;
    assert!(result.is_err());
    let count: i64 = db
        .read(|conn| Ok(conn.query_row("SELECT count(*) FROM instance_state", [], |r| r.get(0))?))
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn backup_is_a_consistent_copy_that_refuses_to_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    let db = open(dir.path());
    db.write(|tx| {
        tx.execute(
            "INSERT INTO instance_state (key, value) VALUES (?1, ?2)",
            params!["pairing", "used"],
        )?;
        Ok(())
    })
    .await
    .unwrap();
    let destination = dir.path().join("copy.db");
    db.backup(&destination).await.unwrap();
    let copy = Connection::open(&destination).unwrap();
    let value: String = copy
        .query_row(
            "SELECT value FROM instance_state WHERE key = 'pairing'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(value, "used");
    let again = db.backup(&destination).await;
    assert!(
        matches!(again, Err(StorageError::BackupExists(_))),
        "{again:?}"
    );
}

#[tokio::test]
async fn instance_values_are_stored_and_replaced() {
    use secure_notify_core::ports::InstanceStore;
    let dir = tempfile::tempdir().unwrap();
    let db = open(dir.path());
    assert_eq!(db.instance_value("pairing_code").await.unwrap(), None);
    db.set_instance_value("pairing_code", "first")
        .await
        .unwrap();
    db.set_instance_value("pairing_code", "second")
        .await
        .unwrap();
    assert_eq!(
        db.instance_value("pairing_code").await.unwrap().as_deref(),
        Some("second")
    );
}

#[tokio::test]
async fn source_state_is_replaced_as_a_whole() {
    use secure_notify_core::sources::{SourceState, SourceStateStore};
    let dir = tempfile::tempdir().unwrap();
    let db = open(dir.path());
    let mut state = SourceState::new();
    state.insert("cursor".to_string(), "1".to_string());
    state.insert("old".to_string(), "x".to_string());
    db.save_source_state("osv", &state).await.unwrap();
    state.remove("old");
    state.insert("cursor".to_string(), "2".to_string());
    db.save_source_state("osv", &state).await.unwrap();
    assert_eq!(db.source_state("osv").await.unwrap(), state);
    assert!(db.source_state("other").await.unwrap().is_empty());
}
