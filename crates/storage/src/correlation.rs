// SPDX-License-Identifier: AGPL-3.0-or-later

//! The correlation transaction: stories, events, identifiers, affected components, entities,
//! relations, and story merges, all written in one write transaction.

use std::collections::BTreeSet;

use rusqlite::{OptionalExtension, Transaction, params};
use secure_notify_core::correlation::{
    CorrelationStore, CorrelationTx, CorrelationWork, EventMeta, EventRecord, IngestCounts,
    StoredEvent, StoryId, StoryRecord,
};
use secure_notify_core::ports::StoreError;
use secure_notify_model::{Affected, Component, EventKind, NormalizedEvent, SeverityLevel};

use crate::time::{from_text, to_text};
use crate::{Database, StorageError};

struct SqlTx<'a, 'b> {
    tx: &'a Transaction<'b>,
}

fn store_error(error: impl std::fmt::Display) -> StoreError {
    StoreError(error.to_string())
}

fn optional_time(text: Option<&str>) -> Result<Option<jiff::Timestamp>, StoreError> {
    text.map(from_text).transpose().map_err(store_error)
}

fn component_key(component: &Component) -> String {
    component
        .purl
        .clone()
        .or_else(|| component.cpe.clone())
        .unwrap_or_default()
}

impl CorrelationTx for SqlTx<'_, '_> {
    fn find_event(&mut self, source: &str, item: &str) -> Result<Option<StoredEvent>, StoreError> {
        self.tx
            .query_row(
                "SELECT id, story_id, content_hash FROM events WHERE source = ?1 AND source_item_id = ?2",
                params![source, item],
                |r| {
                    Ok(StoredEvent {
                        id: r.get(0)?,
                        story: StoryId(r.get(1)?),
                        content_hash: r.get(2)?,
                    })
                },
            )
            .optional()
            .map_err(store_error)
    }

    fn story_by_identifier(
        &mut self,
        kind: &str,
        value: &str,
    ) -> Result<Option<StoryId>, StoreError> {
        self.tx
            .query_row(
                "SELECT story_id FROM story_identifiers WHERE kind = ?1 AND value = ?2",
                params![kind, value],
                |r| Ok(StoryId(r.get(0)?)),
            )
            .optional()
            .map_err(store_error)
    }

    fn story(&mut self, id: StoryId) -> Result<StoryRecord, StoreError> {
        let row = self
            .tx
            .query_row(
                "SELECT kind, title, title_priority, summary, url, published_at, severity_level, \
                 cvss_version, cvss_score, cvss_vector, epss_score, epss_percentile, exploited, \
                 exploit_available, fix_available, first_seen_at, updated_at, eligible_at, \
                 needs_match, backfill, compacted FROM stories WHERE id = ?1",
                [id.0],
                |r| {
                    Ok((
                        (
                            r.get::<_, String>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, i64>(2)?,
                            r.get::<_, Option<String>>(3)?,
                            r.get::<_, Option<String>>(4)?,
                            r.get::<_, Option<String>>(5)?,
                            r.get::<_, String>(6)?,
                        ),
                        (
                            r.get::<_, Option<String>>(7)?,
                            r.get::<_, Option<f64>>(8)?,
                            r.get::<_, Option<String>>(9)?,
                            r.get::<_, Option<f64>>(10)?,
                            r.get::<_, Option<f64>>(11)?,
                        ),
                        (
                            r.get::<_, bool>(12)?,
                            r.get::<_, bool>(13)?,
                            r.get::<_, bool>(14)?,
                            r.get::<_, String>(15)?,
                            r.get::<_, String>(16)?,
                            r.get::<_, Option<String>>(17)?,
                            r.get::<_, bool>(18)?,
                            r.get::<_, bool>(19)?,
                            r.get::<_, bool>(20)?,
                        ),
                    ))
                },
            )
            .map_err(store_error)?;
        let ((kind, title, title_priority, summary, url, published, level), cvss, flags) = row;
        let (cvss_version, cvss_score, cvss_vector, epss_score, epss_percentile) = cvss;
        let (
            exploited,
            exploit_available,
            fix_available,
            first,
            updated,
            eligible,
            needs,
            backfill,
            compacted,
        ) = flags;
        Ok(StoryRecord {
            kind: EventKind::parse(&kind).ok_or_else(|| store_error(format!("kind `{kind}`")))?,
            title,
            title_priority,
            summary,
            url,
            published_at: optional_time(published.as_deref())?,
            severity: SeverityLevel::parse(&level)
                .ok_or_else(|| store_error(format!("severity `{level}`")))?,
            cvss: cvss_version
                .zip(cvss_score)
                .map(|(v, s)| (v, s, cvss_vector)),
            epss: epss_score.zip(epss_percentile),
            exploited,
            exploit_available,
            fix_available,
            first_seen_at: from_text(&first).map_err(store_error)?,
            updated_at: from_text(&updated).map_err(store_error)?,
            eligible_at: optional_time(eligible.as_deref())?,
            needs_match: needs,
            backfill,
            compacted,
        })
    }

    fn story_events(&mut self, id: StoryId) -> Result<Vec<EventRecord>, StoreError> {
        let mut stmt = self
            .tx
            .prepare("SELECT payload, backfill, title_priority FROM events WHERE story_id = ?1 ORDER BY id")
            .map_err(store_error)?;
        let rows = stmt
            .query_map([id.0], |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, bool>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })
            .map_err(store_error)?;
        let mut events = Vec::new();
        for row in rows {
            let (payload, backfill, title_priority) = row.map_err(store_error)?;
            let payload = payload
                .map(|json| serde_json::from_str::<NormalizedEvent>(&json))
                .transpose()
                .map_err(store_error)?;
            events.push(EventRecord {
                payload,
                backfill,
                title_priority,
            });
        }
        Ok(events)
    }

    fn insert_story(&mut self, record: &StoryRecord) -> Result<StoryId, StoreError> {
        self.tx
            .execute(
                "INSERT INTO stories (kind, title, title_priority, severity_level, first_seen_at, \
                 updated_at, backfill) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    record.kind.as_str(),
                    record.title,
                    record.title_priority,
                    record.severity.as_str(),
                    to_text(record.first_seen_at),
                    to_text(record.updated_at),
                    record.backfill
                ],
            )
            .map_err(store_error)?;
        let id = StoryId(self.tx.last_insert_rowid());
        self.update_story(id, record)?;
        Ok(id)
    }

    fn update_story(&mut self, id: StoryId, r: &StoryRecord) -> Result<(), StoreError> {
        let (cvss_version, cvss_score, cvss_vector) = match &r.cvss {
            Some((v, s, vec)) => (Some(v.clone()), Some(*s), vec.clone()),
            None => (None, None, None),
        };
        self.tx
            .execute(
                "UPDATE stories SET kind = ?2, title = ?3, title_priority = ?4, summary = ?5, \
                 url = ?6, published_at = ?7, severity_level = ?8, cvss_version = ?9, \
                 cvss_score = ?10, cvss_vector = ?11, epss_score = ?12, epss_percentile = ?13, \
                 exploited = ?14, exploit_available = ?15, fix_available = ?16, \
                 first_seen_at = ?17, updated_at = ?18, eligible_at = ?19, needs_match = ?20, \
                 backfill = ?21, compacted = ?22 WHERE id = ?1",
                params![
                    id.0,
                    r.kind.as_str(),
                    r.title,
                    r.title_priority,
                    r.summary,
                    r.url,
                    r.published_at.map(to_text),
                    r.severity.as_str(),
                    cvss_version,
                    cvss_score,
                    cvss_vector,
                    r.epss.map(|e| e.0),
                    r.epss.map(|e| e.1),
                    r.exploited,
                    r.exploit_available,
                    r.fix_available,
                    to_text(r.first_seen_at),
                    to_text(r.updated_at),
                    r.eligible_at.map(to_text),
                    r.needs_match,
                    r.backfill,
                    r.compacted
                ],
            )
            .map_err(store_error)?;
        Ok(())
    }

    fn insert_event(
        &mut self,
        story: StoryId,
        event: &NormalizedEvent,
        hash: &str,
        meta: &EventMeta,
    ) -> Result<(), StoreError> {
        let payload = serde_json::to_string(event).map_err(store_error)?;
        let now = to_text(meta.fetched_at);
        self.tx
            .execute(
                "INSERT INTO events (source, source_item_id, story_id, payload, content_hash, \
                 fetched_at, updated_at, backfill, mirror, title_priority) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?7, ?8, ?9)",
                params![
                    event.source,
                    event.source_item_id,
                    story.0,
                    payload,
                    hash,
                    now,
                    meta.backfill,
                    meta.mirror,
                    meta.title_priority
                ],
            )
            .map_err(store_error)?;
        Ok(())
    }

    fn update_event(
        &mut self,
        id: i64,
        event: &NormalizedEvent,
        hash: &str,
        meta: &EventMeta,
    ) -> Result<(), StoreError> {
        let payload = serde_json::to_string(event).map_err(store_error)?;
        self.tx
            .execute(
                "UPDATE events SET payload = ?2, content_hash = ?3, updated_at = ?4, \
                 title_priority = ?5 WHERE id = ?1",
                params![
                    id,
                    payload,
                    hash,
                    to_text(meta.fetched_at),
                    meta.title_priority
                ],
            )
            .map_err(store_error)?;
        Ok(())
    }

    fn add_identifiers(
        &mut self,
        story: StoryId,
        ids: &[(String, String)],
    ) -> Result<(), StoreError> {
        let mut insert = self
            .tx
            .prepare("INSERT OR IGNORE INTO story_identifiers (story_id, kind, value) VALUES (?1, ?2, ?3)")
            .map_err(store_error)?;
        for (kind, value) in ids {
            insert
                .execute(params![story.0, kind, value])
                .map_err(store_error)?;
        }
        Ok(())
    }

    fn replace_affected(
        &mut self,
        story: StoryId,
        affected: &[Affected],
    ) -> Result<(), StoreError> {
        self.tx
            .execute("DELETE FROM story_affected WHERE story_id = ?1", [story.0])
            .map_err(store_error)?;
        let mut insert = self
            .tx
            .prepare("INSERT INTO story_affected (story_id, component, ranges, fixed) VALUES (?1, ?2, ?3, ?4)")
            .map_err(store_error)?;
        for entry in affected {
            let ranges = serde_json::to_string(&entry.ranges).map_err(store_error)?;
            let fixed = serde_json::to_string(&entry.fixed).map_err(store_error)?;
            insert
                .execute(params![
                    story.0,
                    component_key(&entry.component),
                    ranges,
                    fixed
                ])
                .map_err(store_error)?;
        }
        Ok(())
    }

    fn replace_entities(
        &mut self,
        story: StoryId,
        source: &BTreeSet<String>,
        core: &BTreeSet<String>,
    ) -> Result<(), StoreError> {
        self.tx
            .execute("DELETE FROM story_entities WHERE story_id = ?1", [story.0])
            .map_err(store_error)?;
        let mut insert = self
            .tx
            .prepare("INSERT OR IGNORE INTO story_entities (story_id, entity_id, origin) VALUES (?1, ?2, ?3)")
            .map_err(store_error)?;
        for (entities, origin) in [(source, "source"), (core, "core")] {
            for entity in entities {
                insert
                    .execute(params![story.0, entity, origin])
                    .map_err(store_error)?;
            }
        }
        Ok(())
    }

    fn add_relation(&mut self, a: StoryId, b: StoryId) -> Result<(), StoreError> {
        if a == b {
            return Ok(());
        }
        self.tx
            .execute(
                "INSERT OR IGNORE INTO story_relations (story_id, related_story_id) VALUES (?1, ?2), (?2, ?1)",
                params![a.0, b.0],
            )
            .map_err(store_error)?;
        Ok(())
    }

    fn merge_stories(&mut self, from: StoryId, into: StoryId) -> Result<(), StoreError> {
        let (from, into) = (from.0, into.0);
        let statements = [
            "UPDATE events SET story_id = ?2 WHERE story_id = ?1",
            "UPDATE story_identifiers SET story_id = ?2 WHERE story_id = ?1",
            "INSERT OR IGNORE INTO story_relations (story_id, related_story_id) \
             SELECT ?2, related_story_id FROM story_relations WHERE story_id = ?1 AND related_story_id != ?2",
            "INSERT OR IGNORE INTO story_relations (story_id, related_story_id) \
             SELECT story_id, ?2 FROM story_relations WHERE related_story_id = ?1 AND story_id != ?2",
            // The ledger moves: a subscriber who received either story has received the merged one.
            "UPDATE deliveries SET story_id = ?2 WHERE story_id = ?1",
            "DELETE FROM notification_queue WHERE story_id = ?1 AND EXISTS (SELECT 1 FROM \
             notification_queue q WHERE q.story_id = ?2 AND q.subscriber_id = \
             notification_queue.subscriber_id AND q.kind = notification_queue.kind)",
            "UPDATE notification_queue SET story_id = ?2 WHERE story_id = ?1",
            "DELETE FROM mutes WHERE scope = 'story' AND value = CAST(?1 AS TEXT) AND EXISTS \
             (SELECT 1 FROM mutes m WHERE m.scope = 'story' AND m.value = CAST(?2 AS TEXT) \
             AND m.subscriber_id = mutes.subscriber_id)",
            "UPDATE mutes SET value = CAST(?2 AS TEXT) WHERE scope = 'story' AND value = CAST(?1 AS TEXT)",
            "UPDATE story_merges SET into_story_id = ?2 WHERE into_story_id = ?1",
            "INSERT OR REPLACE INTO story_merges (from_story_id, into_story_id) VALUES (?1, ?2)",
        ];
        for sql in statements {
            self.tx
                .execute(sql, params![from, into])
                .map_err(store_error)?;
        }
        self.tx
            .execute("DELETE FROM stories WHERE id = ?1", [from])
            .map_err(store_error)?;
        Ok(())
    }
}

impl CorrelationStore for Database {
    async fn correlate(&self, work: CorrelationWork) -> Result<IngestCounts, StoreError> {
        Ok(self
            .write(move |tx| {
                let mut adapter = SqlTx { tx };
                work(&mut adapter).map_err(|e| StorageError::Corrupt(e.0))
            })
            .await?)
    }
}
