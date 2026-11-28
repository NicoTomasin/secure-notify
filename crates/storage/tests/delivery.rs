// SPDX-License-Identifier: AGPL-3.0-or-later

//! Ledger, queue, endpoints, and mutes on real SQLite (docs/architecture/delivery.md,
//! docs/architecture/deduplication.md#double-check-before-dispatch).

#![allow(clippy::unwrap_used)]

use rusqlite::Connection;
use secure_notify_core::correlation::StoryId;
use secure_notify_core::delivery::{DeliveryStore, QueueItem};
use secure_notify_core::matching::{NoticeKind, Reason, Relevance, Snapshot};
use secure_notify_core::mutes::MuteStore;
use secure_notify_core::subscribers::SubscriberId;
use secure_notify_model::SeverityLevel;
use secure_notify_storage::{Database, OpenOptions};

fn setup() -> (Database, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let options = OpenOptions {
        path: dir.path().join("db.sqlite3"),
        backup_dir: dir.path().join("backups"),
        now: "2026-11-27T12:00:00Z".parse().unwrap(),
        readers: 2,
    };
    let db = Database::open(&options).unwrap().0;
    let conn = Connection::open(dir.path().join("db.sqlite3")).unwrap();
    conn.execute_batch(
        "INSERT INTO subscribers (id, kind, role, created_at) VALUES (1, 'person', 'admin', 'x');
         INSERT INTO endpoints (id, subscriber_id, platform, address, kind, is_primary, created_at)
           VALUES (10, 1, 'test', 'a', 'private', 1, 'x');
         INSERT INTO stories (id, kind, title, title_priority, severity_level, first_seen_at, updated_at, backfill)
           VALUES (1, 'vulnerability', 't', 2, 'high', 'x', 'x', 0);
         INSERT INTO notification_queue (id, subscriber_id, story_id, kind, reason, due_at, created_at)
           VALUES (5, 1, 1, 'new', '{}', '2026-11-27T12:00:00.000000Z', 'x');",
    )
    .unwrap();
    (db, dir)
}

fn count(dir: &tempfile::TempDir, sql: &str) -> i64 {
    Connection::open(dir.path().join("db.sqlite3"))
        .unwrap()
        .query_row(sql, [], |r| r.get(0))
        .unwrap()
}

fn item() -> QueueItem {
    QueueItem {
        id: 5,
        subscriber: SubscriberId(1),
        story: StoryId(1),
        kind: NoticeKind::New,
        reason: Reason {
            relevance: Relevance::ExactVersion,
            entries: Vec::new(),
            entities: Vec::new(),
        },
        attempts: 0,
    }
}

const SNAPSHOT: Snapshot = Snapshot {
    severity: SeverityLevel::High,
    exploited: false,
    exploit_available: false,
    fix_available: false,
};

#[tokio::test]
async fn completing_writes_the_ledger_once_and_removes_the_queue_item() {
    let (db, dir) = setup();
    let now = "2026-11-27T12:01:00Z".parse().unwrap();
    assert!(db.complete(&item(), SNAPSHOT, 10, now).await.unwrap());
    assert_eq!(count(&dir, "SELECT count(*) FROM deliveries"), 1);
    assert_eq!(count(&dir, "SELECT count(*) FROM notification_queue"), 0);
    // A second completion of a `new` item (a retry after a crash) does not duplicate the ledger.
    assert!(!db.complete(&item(), SNAPSHOT, 10, now).await.unwrap());
    assert_eq!(count(&dir, "SELECT count(*) FROM deliveries"), 1);
}

#[tokio::test]
async fn losing_the_last_endpoint_deactivates_the_subscriber_and_drops_its_queue() {
    // subscribers.md#subscribers-and-endpoints.
    let (db, dir) = setup();
    db.endpoint_gone(10).await.unwrap();
    assert_eq!(count(&dir, "SELECT active FROM endpoints WHERE id = 10"), 0);
    assert_eq!(
        count(&dir, "SELECT active FROM subscribers WHERE id = 1"),
        0
    );
    assert_eq!(count(&dir, "SELECT count(*) FROM notification_queue"), 0);
    // Its configuration is kept.
    assert_eq!(count(&dir, "SELECT count(*) FROM subscribers"), 1);
}

#[tokio::test]
async fn expired_mutes_are_not_listed() {
    let (db, _dir) = setup();
    let now = "2026-11-27T12:00:00Z".parse().unwrap();
    let later = "2026-11-28T12:00:00Z".parse().unwrap();
    db.add_mute(SubscriberId(1), "story", "1", Some(later), now)
        .await
        .unwrap();
    db.add_mute(SubscriberId(1), "entity", "tech:node", None, now)
        .await
        .unwrap();
    assert_eq!(db.mutes(SubscriberId(1), now).await.unwrap().len(), 2);
    let after = "2026-11-29T00:00:00Z".parse().unwrap();
    let left = db.mutes(SubscriberId(1), after).await.unwrap();
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].value, "tech:node");
}

#[tokio::test]
async fn completing_a_scan_records_each_story_once_and_clears_the_pending_entries() {
    use secure_notify_core::inventory::ParsedComponent;
    use secure_notify_core::scan::ScanStore;
    let (db, dir) = setup();
    let now = "2026-11-27T12:01:00Z".parse().unwrap();
    let entry = ParsedComponent {
        purl: "pkg:npm/lodash".to_string(),
        version: Some("4.17.20".to_string()),
    };
    db.add_pending_scans(SubscriberId(1), vec![entry.clone()], now)
        .await
        .unwrap();
    // The story was delivered as a notification between the scan and its completion.
    assert!(db.complete(&item(), SNAPSHOT, 10, now).await.unwrap());
    db.complete_scan(
        SubscriberId(1),
        vec![entry],
        vec![(StoryId(1), SNAPSHOT)],
        now,
    )
    .await
    .unwrap();
    assert_eq!(count(&dir, "SELECT count(*) FROM deliveries"), 1);
    assert_eq!(count(&dir, "SELECT count(*) FROM pending_scans"), 0);
}
