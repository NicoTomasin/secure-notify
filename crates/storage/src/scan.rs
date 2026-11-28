// SPDX-License-Identifier: AGPL-3.0-or-later

//! Pending initial scans and their ledger rows (docs/architecture/inventory.md#upload-flow).

use std::collections::BTreeMap;

use rusqlite::{params, params_from_iter};
use secure_notify_core::correlation::StoryId;
use secure_notify_core::inventory::ParsedComponent;
use secure_notify_core::matching::Snapshot;
use secure_notify_core::ports::StoreError;
use secure_notify_core::scan::{ScanHit, ScanStore};
use secure_notify_core::subscribers::{EndpointRef, SubscriberId};

use crate::delivery::{recipient, story_view};
use crate::matching::{candidates, story_facts};
use crate::time::to_text;
use crate::{Database, StorageError};

/// Components per `IN (...)` query, below SQLite's parameter limit.
const CHUNK: usize = 500;

fn corrupt(message: impl std::fmt::Display) -> StorageError {
    StorageError::Corrupt(message.to_string())
}

impl ScanStore for Database {
    async fn add_pending_scans(
        &self,
        subscriber: SubscriberId,
        entries: Vec<ParsedComponent>,
        now: jiff::Timestamp,
    ) -> Result<(), StoreError> {
        Ok(self
            .write(move |tx| {
                let now = to_text(now);
                let mut stmt = tx.prepare(
                    "INSERT OR IGNORE INTO pending_scans (subscriber_id, component, version, created_at) \
                     VALUES (?1, ?2, ?3, ?4)",
                )?;
                for entry in &entries {
                    stmt.execute(params![
                        subscriber.0,
                        entry.purl,
                        entry.version.as_deref().unwrap_or(""),
                        now
                    ])?;
                }
                Ok(())
            })
            .await?)
    }

    async fn pending_scans(&self) -> Result<Vec<(SubscriberId, Vec<ParsedComponent>)>, StoreError> {
        Ok(self
            .read(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT p.subscriber_id, p.component, p.version FROM pending_scans p \
                     JOIN subscribers s ON s.id = p.subscriber_id WHERE s.active = 1 \
                     ORDER BY p.subscriber_id, p.component, p.version",
                )?;
                let rows = stmt.query_map([], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                })?;
                let mut grouped: BTreeMap<i64, Vec<ParsedComponent>> = BTreeMap::new();
                for row in rows {
                    let (subscriber, purl, version) = row?;
                    grouped
                        .entry(subscriber)
                        .or_default()
                        .push(ParsedComponent {
                            purl,
                            version: (!version.is_empty()).then_some(version),
                        });
                }
                Ok(grouped
                    .into_iter()
                    .map(|(s, entries)| (SubscriberId(s), entries))
                    .collect())
            })
            .await?)
    }

    async fn scan_input(
        &self,
        subscriber: SubscriberId,
        components: Vec<String>,
        now: jiff::Timestamp,
    ) -> Result<Vec<ScanHit>, StoreError> {
        let now = to_text(now);
        Ok(self
            .read(move |conn| {
                let mut stories = Vec::new();
                for chunk in components.chunks(CHUNK) {
                    let sql = format!(
                        "SELECT DISTINCT story_id FROM story_affected WHERE component IN ({})",
                        vec!["?"; chunk.len()].join(", ")
                    );
                    let mut stmt = conn.prepare(&sql)?;
                    let rows = stmt.query_map(params_from_iter(chunk), |r| r.get::<_, i64>(0))?;
                    for row in rows {
                        stories.push(row?);
                    }
                }
                stories.sort_unstable();
                stories.dedup();
                let mut hits = Vec::new();
                for story in stories {
                    let (Some(view), Some(facts)) =
                        (story_view(conn, story)?, story_facts(conn, story)?)
                    else {
                        continue;
                    };
                    let Some(candidate) = candidates(conn, story, &facts, &now)?
                        .into_iter()
                        .find(|c| c.subscriber == subscriber)
                    else {
                        continue;
                    };
                    hits.push(ScanHit {
                        view,
                        facts,
                        candidate,
                    });
                }
                Ok(hits)
            })
            .await?)
    }

    async fn scan_recipient(
        &self,
        subscriber: SubscriberId,
    ) -> Result<(Option<(i64, EndpointRef)>, String), StoreError> {
        Ok(self.read(move |conn| recipient(conn, subscriber.0)).await?)
    }

    async fn complete_scan(
        &self,
        subscriber: SubscriberId,
        entries: Vec<ParsedComponent>,
        reported: Vec<(StoryId, Snapshot)>,
        now: jiff::Timestamp,
    ) -> Result<(), StoreError> {
        Ok(self
            .write(move |tx| {
                let now = to_text(now);
                for (story, snapshot) in &reported {
                    let delivered: bool = tx.query_row(
                        "SELECT EXISTS (SELECT 1 FROM deliveries WHERE subscriber_id = ?1 AND story_id = ?2)",
                        params![subscriber.0, story.0],
                        |r| r.get(0),
                    )?;
                    if delivered {
                        continue;
                    }
                    let snapshot = serde_json::to_string(snapshot).map_err(corrupt)?;
                    tx.execute(
                        "INSERT INTO deliveries (subscriber_id, story_id, kind, snapshot, sent_at) \
                         VALUES (?1, ?2, 'scan', ?3, ?4)",
                        params![subscriber.0, story.0, snapshot, now],
                    )?;
                }
                let mut stmt = tx.prepare(
                    "DELETE FROM pending_scans WHERE subscriber_id = ?1 AND component = ?2 AND version = ?3",
                )?;
                for entry in &entries {
                    stmt.execute(params![
                        subscriber.0,
                        entry.purl,
                        entry.version.as_deref().unwrap_or("")
                    ])?;
                }
                Ok(())
            })
            .await?)
    }
}
