// SPDX-License-Identifier: AGPL-3.0-or-later

//! The matcher on real SQLite: grace window, inverted indexes, relevance, and queueing
//! (docs/architecture/matching-and-rules.md, docs/architecture/delivery.md#queue-and-dispatcher).

#![allow(clippy::unwrap_used)]

use std::sync::{Arc, Mutex};

use jiff::{SignedDuration, Timestamp};
use rusqlite::Connection;
use secure_notify_core::correlation::Correlator;
use secure_notify_core::matching::Matcher;
use secure_notify_core::ports::Clock;
use secure_notify_core::taxonomy::Taxonomy;
use secure_notify_storage::{Database, OpenOptions};

struct TestClock(Mutex<Timestamp>);

impl Clock for TestClock {
    fn now(&self) -> Timestamp {
        *self.0.lock().unwrap()
    }
}

/// The real GHSA-35jh-r3h4-6jhm range for lodash, as the OSV source produces it.
const ADVISORY: &str = r#"{"schema_version":"1","source":"osv","source_item_id":"GHSA-35jh-r3h4-6jhm",
"title":"Command Injection in lodash","kind":"vulnerability",
"identifiers":{"cve":["CVE-2021-23337"],"ghsa":["GHSA-35jh-r3h4-6jhm"]},
"affected":[{"component":{"purl":"pkg:npm/lodash"},"ranges":[{"type":"semver","events":[{"introduced":"0"},{"fixed":"4.17.21"}]}],"fixed":["4.17.21"]}],
"entities":{"technologies":["tech:node"]},
"severity":{"level":"high"}}"#;

#[tokio::test]
async fn stories_reach_relevant_subscribers_after_the_grace_window() {
    let dir = tempfile::tempdir().unwrap();
    let options = OpenOptions {
        path: dir.path().join("db.sqlite3"),
        backup_dir: dir.path().join("backups"),
        now: "2026-11-27T12:00:00Z".parse().unwrap(),
        readers: 2,
    };
    let db = Database::open(&options).unwrap().0;
    let conn = Connection::open(dir.path().join("db.sqlite3")).unwrap();
    // alice uses an affected version, bob the fixed one, carol watches Node.js, dave nothing.
    conn.execute_batch(
        "INSERT INTO subscribers (id, kind, role, created_at) VALUES
           (1, 'person', 'admin', 'x'), (2, 'person', 'member', 'x'),
           (3, 'person', 'member', 'x'), (4, 'person', 'member', 'x');
         INSERT INTO inventory_entries (subscriber_id, component, version, last_seen_at) VALUES
           (1, 'pkg:npm/lodash', '4.17.20', 'x'), (2, 'pkg:npm/lodash', '4.17.21', 'x');
         INSERT INTO watches (subscriber_id, entity_id, origin, created_at) VALUES
           (3, 'tech:node', 'manual', 'x');",
    )
    .unwrap();
    let clock = Arc::new(TestClock(Mutex::new(
        "2026-11-27T12:00:00Z".parse().unwrap(),
    )));
    let correlator = Correlator::new(
        db.clone(),
        Arc::new(Taxonomy::default()),
        clock.clone(),
        SignedDuration::from_mins(5),
    );
    let event = secure_notify_model::parse_event(ADVISORY.as_bytes()).unwrap();
    correlator
        .ingest_events(true, true, vec![event], false)
        .await
        .unwrap();
    let matcher = Matcher::new(db.clone(), clock.clone());

    // Inside the grace window nothing is matched.
    assert_eq!(matcher.run_once().await.unwrap(), 0);
    *clock.0.lock().unwrap() = "2026-11-27T12:06:00Z".parse().unwrap();
    assert_eq!(matcher.run_once().await.unwrap(), 1);

    let mut stmt = conn
        .prepare(
            "SELECT subscriber_id, kind, reason FROM notification_queue ORDER BY subscriber_id",
        )
        .unwrap();
    let queued: Vec<(i64, String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let subscribers: Vec<i64> = queued.iter().map(|q| q.0).collect();
    // carol's technology relevance needs severity high (default rule 8): the story is high.
    assert_eq!(subscribers, [1, 3]);
    assert!(
        queued[0].2.contains("\"relevance\":\"exact-version\""),
        "{}",
        queued[0].2
    );
    assert!(queued[0].2.contains("4.17.20"));
    assert!(queued[1].2.contains("\"relevance\":\"technology\""));
    let needs: i64 = conn
        .query_row("SELECT needs_match FROM stories", [], |r| r.get(0))
        .unwrap();
    assert_eq!(needs, 0);
    // Matching again finds nothing to do.
    assert_eq!(matcher.run_once().await.unwrap(), 0);
}
