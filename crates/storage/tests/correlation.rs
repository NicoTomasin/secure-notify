// SPDX-License-Identifier: AGPL-3.0-or-later

//! Correlation and retention on real SQLite with the real migrations
//! (docs/architecture/deduplication.md).

// Test helpers take JSON by value for readability.
#![allow(
    clippy::unwrap_used,
    clippy::needless_pass_by_value,
    clippy::bool_assert_comparison
)]

use std::sync::{Arc, Mutex};

use jiff::{SignedDuration, Timestamp};
use rusqlite::Connection;
use secure_notify_core::correlation::{Correlator, IngestCounts};
use secure_notify_core::ports::Clock;
use secure_notify_core::retention;
use secure_notify_core::taxonomy::Taxonomy;
use secure_notify_model::NormalizedEvent;
use secure_notify_storage::{Database, OpenOptions};
use serde_json::{Value, json};

struct TestClock(Mutex<Timestamp>);

impl Clock for TestClock {
    fn now(&self) -> Timestamp {
        *self.0.lock().unwrap()
    }
}

struct Fixture {
    db: Database,
    correlator: Correlator<Database>,
    clock: Arc<TestClock>,
    dir: tempfile::TempDir,
}

const GRACE: SignedDuration = SignedDuration::from_mins(5);

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let options = OpenOptions {
            path: dir.path().join("db.sqlite3"),
            backup_dir: dir.path().join("backups"),
            now: "2026-11-27T12:00:00Z".parse().unwrap(),
            readers: 2,
        };
        let db = Database::open(&options).unwrap().0;
        let clock = Arc::new(TestClock(Mutex::new(
            "2026-11-27T12:00:00Z".parse().unwrap(),
        )));
        let correlator = Correlator::new(
            db.clone(),
            Arc::new(Taxonomy::default()),
            clock.clone(),
            GRACE,
        );
        Self {
            db,
            correlator,
            clock,
            dir,
        }
    }

    async fn ingest(&self, events: Vec<Value>, backfill: bool) -> IngestCounts {
        let events: Vec<NormalizedEvent> = events
            .into_iter()
            .map(|v| secure_notify_model::parse_event(v.to_string().as_bytes()).unwrap())
            .collect();
        self.correlator
            .ingest_events(true, false, events, backfill)
            .await
            .unwrap()
    }

    fn sql(&self) -> Connection {
        Connection::open(self.dir.path().join("db.sqlite3")).unwrap()
    }

    fn count(&self, sql: &str) -> i64 {
        self.sql().query_row(sql, [], |r| r.get(0)).unwrap()
    }

    fn advance(&self, by: SignedDuration) {
        let mut now = self.clock.0.lock().unwrap();
        *now = now.checked_add(by).unwrap();
    }
}

fn event(source: &str, item: &str, extra: Value) -> Value {
    let mut base = json!({
        "schema_version": "1", "source": source, "source_item_id": item,
        "title": format!("{source} {item}"), "kind": "vulnerability"
    });
    for (key, value) in extra.as_object().unwrap() {
        base[key] = value.clone();
    }
    base
}

#[tokio::test]
async fn refetching_the_same_item_creates_nothing_new() {
    // overview.md#cross-cutting-concerns: re-fetching never duplicates; deduplication.md: an
    // event is updated only when its content changed.
    let f = Fixture::new();
    let e = event("osv", "GHSA-35jh-r3h4-6jhm", json!({}));
    f.ingest(vec![e.clone()], false).await;
    let again = f.ingest(vec![e.clone()], false).await;
    assert_eq!((again.new, again.changed, again.unchanged), (0, 0, 1));
    assert_eq!(f.count("SELECT count(*) FROM events"), 1);
    assert_eq!(f.count("SELECT count(*) FROM stories"), 1);
    let mut changed = e;
    changed["title"] = json!("Command Injection in lodash");
    f.ingest(vec![changed], false).await;
    assert_eq!(f.count("SELECT count(*) FROM events"), 1);
    let title: String = f
        .sql()
        .query_row("SELECT title FROM stories", [], |r| r.get(0))
        .unwrap();
    assert_eq!(title, "Command Injection in lodash");
}

#[tokio::test]
async fn aliases_resolve_reports_from_different_sources_to_one_story() {
    // deduplication.md#correlation-signals: identifiers with aliases (CVE ↔ GHSA).
    let f = Fixture::new();
    let ids = json!({"identifiers": {"cve": ["CVE-2021-23337"], "ghsa": ["GHSA-35jh-r3h4-6jhm"]}});
    f.ingest(vec![event("osv", "GHSA-35jh-r3h4-6jhm", ids)], false)
        .await;
    f.ingest(vec![event("kev", "CVE-2021-23337", json!({"kind": "exploit", "identifiers": {"cve": ["CVE-2021-23337"]}, "severity": {"exploited": true}}))], false).await;
    assert_eq!(f.count("SELECT count(*) FROM stories"), 1);
    assert_eq!(f.count("SELECT count(*) FROM events"), 2);
    // The exploit event sets `exploited` on the vulnerability story; the kind stays vulnerability.
    let (kind, exploited): (String, bool) = f
        .sql()
        .query_row("SELECT kind, exploited FROM stories", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    assert_eq!((kind.as_str(), exploited), ("vulnerability", true));
}

#[tokio::test]
async fn an_event_with_identifiers_of_two_stories_merges_them_with_their_ledger() {
    // deduplication.md#story-merges: stories merge, and the ledger merges too.
    let f = Fixture::new();
    f.ingest(
        vec![event(
            "a",
            "1",
            json!({"identifiers": {"cve": ["CVE-2026-0001"]}}),
        )],
        false,
    )
    .await;
    f.ingest(
        vec![event(
            "b",
            "2",
            json!({"identifiers": {"ghsa": ["GHSA-aaaa-bbbb-cccc"]}}),
        )],
        false,
    )
    .await;
    assert_eq!(f.count("SELECT count(*) FROM stories"), 2);
    let conn = f.sql();
    conn.execute_batch(
        "INSERT INTO subscribers (id, kind, role, created_at) VALUES (1, 'person', 'admin', 'x');
         INSERT INTO deliveries (subscriber_id, story_id, kind, snapshot, sent_at)
         SELECT 1, id, 'new', '{}', 'x' FROM stories WHERE id = (SELECT max(id) FROM stories);",
    )
    .unwrap();
    drop(conn);
    f.ingest(
        vec![event(
            "c",
            "3",
            json!({"identifiers": {"cve": ["CVE-2026-0001"], "ghsa": ["GHSA-aaaa-bbbb-cccc"]}}),
        )],
        false,
    )
    .await;
    assert_eq!(f.count("SELECT count(*) FROM stories"), 1);
    assert_eq!(f.count("SELECT count(*) FROM story_merges"), 1);
    // The delivery of the merged-away story now points to the surviving one.
    assert_eq!(
        f.count("SELECT count(*) FROM deliveries d JOIN stories s ON s.id = d.story_id"),
        1
    );
    assert_eq!(
        f.count("SELECT count(*) FROM story_identifiers WHERE kind IN ('cve', 'ghsa')"),
        2
    );
}

#[tokio::test]
async fn families_are_never_merged_but_linked_as_related() {
    // deduplication.md#kind-families.
    let f = Fixture::new();
    f.ingest(
        vec![event(
            "osv",
            "1",
            json!({"identifiers": {"cve": ["CVE-2026-0002"]}}),
        )],
        false,
    )
    .await;
    f.ingest(vec![event("breaches", "x", json!({"kind": "breach", "identifiers": {"cve": ["CVE-2026-0002"], "other": ["breach:example:x"]}}))], false).await;
    assert_eq!(f.count("SELECT count(*) FROM stories"), 2);
    assert_eq!(f.count("SELECT count(*) FROM story_relations"), 2);
}

#[tokio::test]
async fn linked_identifiers_join_one_story_but_never_merge_several() {
    let f = Fixture::new();
    f.ingest(
        vec![event(
            "osv",
            "1",
            json!({"identifiers": {"ghsa": ["GHSA-aaaa-bbbb-cccc"]}}),
        )],
        false,
    )
    .await;
    f.ingest(
        vec![event(
            "osv",
            "2",
            json!({"identifiers": {"ghsa": ["GHSA-dddd-eeee-ffff"]}}),
        )],
        false,
    )
    .await;
    // A blog post linking one advisory joins it.
    f.ingest(
        vec![event(
            "blog",
            "p1",
            json!({"kind": "news", "links": ["https://github.com/advisories/GHSA-aaaa-bbbb-cccc"]}),
        )],
        false,
    )
    .await;
    assert_eq!(f.count("SELECT count(*) FROM stories"), 2);
    // A post mentioning both advisories does not merge them.
    f.ingest(vec![event("blog", "p2", json!({"kind": "news", "summary": "Covers GHSA-aaaa-bbbb-cccc and GHSA-dddd-eeee-ffff."}))], false).await;
    assert_eq!(f.count("SELECT count(*) FROM stories"), 3);
    assert_eq!(f.count("SELECT count(*) FROM story_merges"), 0);
}

#[tokio::test]
async fn the_same_canonical_url_means_the_same_story() {
    let f = Fixture::new();
    f.ingest(
        vec![event(
            "feed-a",
            "1",
            json!({"kind": "news", "url": "https://Example.com/post/?utm_source=rss"}),
        )],
        false,
    )
    .await;
    f.ingest(
        vec![event(
            "feed-b",
            "9",
            json!({"kind": "news", "url": "https://example.com/post#comments"}),
        )],
        false,
    )
    .await;
    assert_eq!(f.count("SELECT count(*) FROM stories"), 1);
}

#[tokio::test]
async fn backfilled_stories_are_not_handed_to_the_matcher() {
    // deduplication.md#backfill and #grace-window.
    let f = Fixture::new();
    f.ingest(vec![event("osv", "old", json!({}))], true).await;
    f.ingest(vec![event("osv", "new", json!({}))], false).await;
    let rows: Vec<(String, bool, Option<String>)> = {
        let conn = f.sql();
        let mut stmt = conn
            .prepare("SELECT s.title, s.needs_match, s.eligible_at FROM stories s ORDER BY id")
            .unwrap();
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    };
    assert_eq!(rows[0].1, false, "backfilled story must not be matched");
    assert_eq!(rows[1].1, true);
    // eligible_at = now + grace window (5 minutes).
    assert_eq!(rows[1].2.as_deref(), Some("2026-11-27T12:05:00.000000Z"));
}

#[tokio::test]
async fn only_material_changes_from_non_backfill_events_reach_the_matcher() {
    let f = Fixture::new();
    let ids = json!({"identifiers": {"cve": ["CVE-2026-0003"]}});
    f.ingest(vec![event("osv", "1", ids.clone())], true).await;
    // Another report of the same thing, without a material change: nothing to notify.
    f.ingest(vec![event("feed", "a", ids.clone())], false).await;
    assert_eq!(f.count("SELECT needs_match FROM stories"), 0);
    // It becomes exploited: an update worth matching.
    f.ingest(vec![event("kev", "CVE-2026-0003", json!({"kind": "exploit", "identifiers": {"cve": ["CVE-2026-0003"]}, "severity": {"exploited": true}}))], false).await;
    assert_eq!(f.count("SELECT needs_match FROM stories"), 1);
}

#[tokio::test]
async fn retention_keeps_minimal_records_and_mirrors_and_rebuilds_compacted_stories() {
    // deduplication.md#retention.
    let f = Fixture::new();
    let mirror_event: NormalizedEvent = secure_notify_model::parse_event(
        event(
            "osv",
            "m1",
            json!({"identifiers": {"cve": ["CVE-2026-0005"]}, "summary": "kept"}),
        )
        .to_string()
        .as_bytes(),
    )
    .unwrap();
    f.correlator
        .ingest_events(true, true, vec![mirror_event], true)
        .await
        .unwrap();
    f.ingest(vec![event("feed", "1", json!({"identifiers": {"cve": ["CVE-2026-0004"]}, "summary": "details", "affected": [{"component": {"purl": "pkg:npm/x"}}]}))], false).await;
    f.advance(SignedDuration::from_hours(8 * 24));
    let counts = retention::run(
        &f.db,
        f.clock.now(),
        SignedDuration::from_hours(7 * 24),
        SignedDuration::from_hours(365 * 24),
    )
    .await
    .unwrap();
    assert_eq!(
        (
            counts.payloads_expired,
            counts.stories_compacted,
            counts.stories_deleted
        ),
        (1, 1, 0)
    );
    // The compacted story keeps its identifiers but loses details; the mirror story keeps all.
    assert_eq!(
        f.count("SELECT count(*) FROM story_identifiers WHERE value = 'CVE-2026-0004'"),
        1
    );
    assert_eq!(
        f.count("SELECT count(*) FROM stories WHERE compacted = 1 AND summary IS NULL"),
        1
    );
    assert_eq!(f.count("SELECT count(*) FROM story_affected"), 0);
    assert_eq!(
        f.count("SELECT count(*) FROM stories WHERE summary = 'kept' AND compacted = 0"),
        1
    );
    // A new report of the compacted story rebuilds it under the same identity.
    let id_before = f.count("SELECT story_id FROM story_identifiers WHERE value = 'CVE-2026-0004'");
    // As if the matcher had processed every story already.
    f.sql()
        .execute("UPDATE stories SET needs_match = 0", [])
        .unwrap();
    f.ingest(
        vec![event(
            "other",
            "2",
            json!({"identifiers": {"cve": ["CVE-2026-0004"]}, "summary": "again"}),
        )],
        false,
    )
    .await;
    assert_eq!(
        f.count(&format!(
            "SELECT compacted FROM stories WHERE id = {id_before}"
        )),
        0
    );
    // The rebuilt story goes back to the matcher, which compares it with the ledger.
    assert_eq!(
        f.count(&format!(
            "SELECT needs_match FROM stories WHERE id = {id_before}"
        )),
        1
    );
    assert_eq!(
        f.count("SELECT count(*) FROM stories WHERE summary = 'again'"),
        1
    );
    // After the deduplication period, a compacted record is deleted.
    f.advance(SignedDuration::from_hours(8 * 24));
    retention::run(
        &f.db,
        f.clock.now(),
        SignedDuration::from_hours(7 * 24),
        SignedDuration::from_hours(365 * 24),
    )
    .await
    .unwrap();
    f.advance(SignedDuration::from_hours(366 * 24));
    let counts = retention::run(
        &f.db,
        f.clock.now(),
        SignedDuration::from_hours(7 * 24),
        SignedDuration::from_hours(365 * 24),
    )
    .await
    .unwrap();
    assert_eq!(counts.stories_deleted, 1);
    assert_eq!(
        f.count("SELECT count(*) FROM stories"),
        1,
        "only the mirror story remains"
    );
}

#[tokio::test]
async fn withdrawn_items_disappear_with_stories_left_empty() {
    // deduplication.md#retention: mirrored records are removed when the source withdraws them.
    let f = Fixture::new();
    let ids = json!({"identifiers": {"cve": ["CVE-2026-0006"]}});
    f.ingest(vec![event("osv", "GHSA-aaaa-bbbb-cccc", json!({}))], true)
        .await;
    f.ingest(
        vec![event("osv", "OSV-2", ids.clone()), event("feed", "x", ids)],
        true,
    )
    .await;
    let removed = f
        .correlator
        .withdraw_items(
            "osv",
            vec![
                "GHSA-aaaa-bbbb-cccc".to_string(),
                "OSV-2".to_string(),
                "unknown".to_string(),
            ],
        )
        .await
        .unwrap();
    assert_eq!(removed, 2);
    // The first story had only the withdrawn event; the second keeps its other report.
    assert_eq!(f.count("SELECT count(*) FROM stories"), 1);
    assert_eq!(f.count("SELECT count(*) FROM events"), 1);
}
