// SPDX-License-Identifier: AGPL-3.0-or-later

//! The queue, the delivery ledger, delivery attempts, and endpoint deactivation.

use rusqlite::{OptionalExtension, params};
use secure_notify_core::correlation::StoryId;
use secure_notify_core::delivery::{DeliveryStore, Prepared, QueueItem, StoryView};
use secure_notify_core::matching::{NoticeKind, Reason, Snapshot};
use secure_notify_core::ports::StoreError;
use secure_notify_core::subscribers::{EndpointRef, SubscriberId};
use secure_notify_model::{EventKind, SeverityLevel};

use crate::time::to_text;
use crate::{Database, StorageError};

fn corrupt(message: impl std::fmt::Display) -> StorageError {
    StorageError::Corrupt(message.to_string())
}

impl DeliveryStore for Database {
    async fn due_items(
        &self,
        now: jiff::Timestamp,
        limit: usize,
    ) -> Result<Vec<QueueItem>, StoreError> {
        let now = to_text(now);
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        Ok(self
            .read(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT q.id, q.subscriber_id, q.story_id, q.kind, q.reason, q.attempts \
                     FROM notification_queue q JOIN subscribers s ON s.id = q.subscriber_id \
                     WHERE q.due_at <= ?1 AND s.active = 1 ORDER BY q.due_at, q.id LIMIT ?2",
                )?;
                let rows = stmt.query_map(params![now, limit], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, i64>(1)?,
                        r.get::<_, i64>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, String>(4)?,
                        r.get::<_, u32>(5)?,
                    ))
                })?;
                let mut items = Vec::new();
                for row in rows {
                    let (id, subscriber, story, kind, reason, attempts) = row?;
                    items.push(QueueItem {
                        id,
                        subscriber: SubscriberId(subscriber),
                        story: StoryId(story),
                        kind: if kind == "update" {
                            NoticeKind::Update
                        } else {
                            NoticeKind::New
                        },
                        reason: serde_json::from_str::<Reason>(&reason).map_err(corrupt)?,
                        attempts,
                    });
                }
                Ok(items)
            })
            .await?)
    }

    async fn prepare(&self, item: &QueueItem) -> Result<Option<Prepared>, StoreError> {
        let (story, subscriber) = (item.story.0, item.subscriber.0);
        Ok(self
            .read(move |conn| {
                let row = conn
                    .query_row(
                        "SELECT kind, title, url, severity_level, exploited, exploit_available, \
                         fix_available, cvss_version, cvss_score, epss_score, summary \
                         FROM stories WHERE id = ?1",
                        [story],
                        |r| {
                            Ok((
                                r.get::<_, String>(0)?,
                                r.get::<_, String>(1)?,
                                r.get::<_, Option<String>>(2)?,
                                r.get::<_, String>(3)?,
                                r.get::<_, bool>(4)?,
                                r.get::<_, bool>(5)?,
                                r.get::<_, bool>(6)?,
                                r.get::<_, Option<String>>(7)?,
                                r.get::<_, Option<f64>>(8)?,
                                r.get::<_, Option<f64>>(9)?,
                                r.get::<_, Option<String>>(10)?,
                            ))
                        },
                    )
                    .optional()?;
                let Some((kind, title, url, level, exploited, exploit_available, fix, cv, cs, epss, summary)) =
                    row
                else {
                    return Ok(None);
                };
                let mut stmt = conn.prepare(
                    "SELECT entity_id FROM story_entities WHERE story_id = ?1 ORDER BY entity_id",
                )?;
                let entities: Vec<String> = stmt
                    .query_map([story], |r| r.get(0))?
                    .collect::<Result<_, _>>()?;
                let sources: i64 = conn.query_row(
                    "SELECT count(DISTINCT source) FROM events WHERE story_id = ?1",
                    [story],
                    |r| r.get(0),
                )?;
                let delivered: Option<String> = conn
                    .query_row(
                        "SELECT snapshot FROM deliveries WHERE subscriber_id = ?1 AND story_id = ?2 \
                         ORDER BY id DESC LIMIT 1",
                        params![subscriber, story],
                        |r| r.get(0),
                    )
                    .optional()?;
                let endpoint = conn
                    .query_row(
                        "SELECT id, platform, address FROM endpoints \
                         WHERE subscriber_id = ?1 AND active = 1 ORDER BY is_primary DESC, id LIMIT 1",
                        [subscriber],
                        |r| {
                            Ok((
                                r.get::<_, i64>(0)?,
                                EndpointRef {
                                    platform: r.get(1)?,
                                    address: r.get(2)?,
                                },
                            ))
                        },
                    )
                    .optional()?;
                let language: String = conn.query_row(
                    "SELECT language FROM subscribers WHERE id = ?1",
                    [subscriber],
                    |r| r.get(0),
                )?;
                Ok(Some(Prepared {
                    story: StoryView {
                        id: StoryId(story),
                        kind: EventKind::parse(&kind).ok_or_else(|| corrupt(&kind))?,
                        title,
                        url,
                        severity: SeverityLevel::parse(&level).ok_or_else(|| corrupt(&level))?,
                        exploited,
                        exploit_available,
                        fix_available: fix,
                        cvss: cv.zip(cs),
                        epss,
                        summary,
                        entities,
                        sources: usize::try_from(sources).unwrap_or(0),
                    },
                    delivered: delivered
                        .map(|s| serde_json::from_str::<Snapshot>(&s))
                        .transpose()
                        .map_err(corrupt)?,
                    endpoint,
                    language,
                }))
            })
            .await?)
    }

    async fn complete(
        &self,
        item: &QueueItem,
        snapshot: Snapshot,
        endpoint: i64,
        now: jiff::Timestamp,
    ) -> Result<bool, StoreError> {
        let item = item.clone();
        Ok(self
            .write(move |tx| {
                let now = to_text(now);
                // The double check inside the transaction: a `new` story already in the ledger
                // is not recorded twice.
                let delivered: bool = tx.query_row(
                    "SELECT EXISTS (SELECT 1 FROM deliveries WHERE subscriber_id = ?1 AND story_id = ?2)",
                    params![item.subscriber.0, item.story.0],
                    |r| r.get(0),
                )?;
                let record = !(item.kind == NoticeKind::New && delivered);
                if record {
                    let snapshot = serde_json::to_string(&snapshot).map_err(corrupt)?;
                    tx.execute(
                        "INSERT INTO deliveries (subscriber_id, story_id, kind, snapshot, sent_at) \
                         VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![item.subscriber.0, item.story.0, item.kind.as_str(), snapshot, now],
                    )?;
                }
                tx.execute(
                    "INSERT INTO delivery_attempts (queue_item_id, endpoint_id, attempt, status, at) \
                     VALUES (?1, ?2, ?3, 'sent', ?4)",
                    params![item.id, endpoint, item.attempts + 1, now],
                )?;
                tx.execute("DELETE FROM notification_queue WHERE id = ?1", [item.id])?;
                Ok(record)
            })
            .await?)
    }

    async fn retry_later(
        &self,
        item: &QueueItem,
        endpoint: i64,
        error: &str,
        retry_at: jiff::Timestamp,
        now: jiff::Timestamp,
    ) -> Result<(), StoreError> {
        let (id, attempt, error) = (item.id, item.attempts + 1, error.to_string());
        Ok(self
            .write(move |tx| {
                tx.execute(
                    "UPDATE notification_queue SET attempts = ?2, due_at = ?3 WHERE id = ?1",
                    params![id, attempt, to_text(retry_at)],
                )?;
                tx.execute(
                    "INSERT INTO delivery_attempts (queue_item_id, endpoint_id, attempt, status, error, at) \
                     VALUES (?1, ?2, ?3, 'failed', ?4, ?5)",
                    params![id, endpoint, attempt, error, to_text(now)],
                )?;
                Ok(())
            })
            .await?)
    }

    async fn drop_item(&self, item: &QueueItem) -> Result<(), StoreError> {
        let id = item.id;
        Ok(self
            .write(move |tx| {
                tx.execute("DELETE FROM notification_queue WHERE id = ?1", [id])?;
                Ok(())
            })
            .await?)
    }

    async fn endpoint_gone(&self, endpoint: i64) -> Result<(), StoreError> {
        Ok(self
            .write(move |tx| {
                tx.execute("UPDATE endpoints SET active = 0 WHERE id = ?1", [endpoint])?;
                let subscriber: i64 = tx.query_row(
                    "SELECT subscriber_id FROM endpoints WHERE id = ?1",
                    [endpoint],
                    |r| r.get(0),
                )?;
                let active: bool = tx.query_row(
                    "SELECT EXISTS (SELECT 1 FROM endpoints WHERE subscriber_id = ?1 AND active = 1)",
                    [subscriber],
                    |r| r.get(0),
                )?;
                if !active {
                    tx.execute("UPDATE subscribers SET active = 0 WHERE id = ?1", [subscriber])?;
                    tx.execute(
                        "DELETE FROM notification_queue WHERE subscriber_id = ?1",
                        [subscriber],
                    )?;
                }
                Ok(())
            })
            .await?)
    }
}

impl secure_notify_core::mutes::MuteStore for Database {
    async fn add_mute(
        &self,
        subscriber: SubscriberId,
        scope: &str,
        value: &str,
        until: Option<jiff::Timestamp>,
        now: jiff::Timestamp,
    ) -> Result<(), StoreError> {
        let (scope, value) = (scope.to_string(), value.to_string());
        Ok(self
            .write(move |tx| {
                tx.execute(
                    "INSERT INTO mutes (subscriber_id, scope, value, until, created_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT (subscriber_id, scope, value) \
                     DO UPDATE SET until = excluded.until",
                    params![subscriber.0, scope, value, until.map(to_text), to_text(now)],
                )?;
                Ok(())
            })
            .await?)
    }

    async fn mutes(
        &self,
        subscriber: SubscriberId,
        now: jiff::Timestamp,
    ) -> Result<Vec<secure_notify_core::mutes::MuteEntry>, StoreError> {
        let now = to_text(now);
        Ok(self
            .read(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, scope, value, until FROM mutes WHERE subscriber_id = ?1 \
                     AND (until IS NULL OR until > ?2) ORDER BY id",
                )?;
                let rows = stmt.query_map(params![subscriber.0, now], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, Option<String>>(3)?,
                    ))
                })?;
                let mut list = Vec::new();
                for row in rows {
                    let (id, scope, value, until) = row?;
                    list.push(secure_notify_core::mutes::MuteEntry {
                        id,
                        scope,
                        value,
                        until: until.as_deref().map(crate::time::from_text).transpose()?,
                    });
                }
                Ok(list)
            })
            .await?)
    }

    async fn remove_mute(&self, subscriber: SubscriberId, id: i64) -> Result<bool, StoreError> {
        Ok(self
            .write(move |tx| {
                Ok(tx.execute(
                    "DELETE FROM mutes WHERE subscriber_id = ?1 AND id = ?2",
                    params![subscriber.0, id],
                )? > 0)
            })
            .await?)
    }
}

impl secure_notify_core::delivery::DetailsStore for Database {
    async fn story_details(
        &self,
        story: StoryId,
    ) -> Result<Option<secure_notify_core::delivery::StoryDetails>, StoreError> {
        Ok(self
            .read(move |conn| {
                let id: i64 = conn
                    .query_row(
                        "SELECT into_story_id FROM story_merges WHERE from_story_id = ?1",
                        [story.0],
                        |r| r.get(0),
                    )
                    .optional()?
                    .unwrap_or(story.0);
                let Some(title) = conn
                    .query_row("SELECT title FROM stories WHERE id = ?1", [id], |r| r.get::<_, String>(0))
                    .optional()?
                else {
                    return Ok(None);
                };
                let strings = |sql: &str| -> Result<Vec<String>, StorageError> {
                    let mut stmt = conn.prepare(sql)?;
                    let rows = stmt.query_map([id], |r| r.get::<_, String>(0))?;
                    Ok(rows.collect::<Result<_, _>>()?)
                };
                let mut stmt = conn
                    .prepare("SELECT component, fixed FROM story_affected WHERE story_id = ?1 ORDER BY component")?;
                let affected = stmt
                    .query_map([id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
                    .map(|row| {
                        let (component, fixed) = row?;
                        let fixed: Vec<String> = serde_json::from_str(&fixed).map_err(corrupt)?;
                        Ok((component, fixed))
                    })
                    .collect::<Result<Vec<_>, StorageError>>()?;
                Ok(Some(secure_notify_core::delivery::StoryDetails {
                    title,
                    identifiers: strings(
                        "SELECT kind || ':' || value FROM story_identifiers \
                         WHERE story_id = ?1 AND kind != 'url' ORDER BY kind, value",
                    )?,
                    sources: strings("SELECT DISTINCT source FROM events WHERE story_id = ?1 ORDER BY source")?,
                    affected,
                    related: strings(
                        "SELECT s.title FROM story_relations r JOIN stories s ON s.id = r.related_story_id \
                         WHERE r.story_id = ?1 ORDER BY s.id",
                    )?,
                }))
            })
            .await?)
    }
}
