// SPDX-License-Identifier: AGPL-3.0-or-later

//! Matching input through the inverted indexes, and queueing.

use std::collections::BTreeMap;

use rusqlite::{Connection, params, params_from_iter};
use secure_notify_core::correlation::StoryId;
use secure_notify_core::matching::{
    Candidate, Enqueue, MatchStore, Mute, Rule, RuleAction, RuleMatch, Snapshot, StoryFacts,
};
use secure_notify_core::ports::StoreError;
use secure_notify_core::subscribers::SubscriberId;
use secure_notify_model::{Affected, Component, EventKind, SeverityLevel};

use crate::time::{from_text, to_text};
use crate::{Database, StorageError};

fn corrupt(message: impl std::fmt::Display) -> StorageError {
    StorageError::Corrupt(message.to_string())
}

fn placeholders(n: usize) -> String {
    vec!["?"; n].join(", ")
}

fn story_facts(conn: &Connection, story: i64) -> Result<Option<StoryFacts>, StorageError> {
    let row = conn.query_row(
        "SELECT kind, severity_level, exploited, exploit_available, fix_available, epss_score, \
         eligible_at, updated_at FROM stories WHERE id = ?1",
        [story],
        |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, bool>(2)?,
                r.get::<_, bool>(3)?,
                r.get::<_, bool>(4)?,
                r.get::<_, Option<f64>>(5)?,
                r.get::<_, Option<String>>(6)?,
                r.get::<_, String>(7)?,
            ))
        },
    );
    let (kind, level, exploited, exploit_available, fix_available, epss, eligible, updated) =
        match row {
            Ok(row) => row,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(e) => return Err(e.into()),
        };
    let column = |sql: &str| -> Result<Vec<String>, StorageError> {
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([story], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    };
    let mut affected = Vec::new();
    let mut stmt =
        conn.prepare("SELECT component, ranges, fixed FROM story_affected WHERE story_id = ?1")?;
    let rows = stmt.query_map([story], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })?;
    for row in rows {
        let (component, ranges, fixed) = row?;
        let is_cpe = component.starts_with("cpe:");
        affected.push(Affected {
            component: Component {
                purl: (!is_cpe).then(|| component.clone()),
                cpe: is_cpe.then_some(component),
            },
            ranges: serde_json::from_str(&ranges).map_err(corrupt)?,
            fixed: serde_json::from_str(&fixed).map_err(corrupt)?,
        });
    }
    Ok(Some(StoryFacts {
        kind: EventKind::parse(&kind).ok_or_else(|| corrupt(format!("kind {kind}")))?,
        severity: SeverityLevel::parse(&level).ok_or_else(|| corrupt(format!("level {level}")))?,
        exploited,
        exploit_available,
        fix_available,
        epss,
        sources: column("SELECT DISTINCT source FROM events WHERE story_id = ?1 ORDER BY source")?,
        entities: column(
            "SELECT entity_id FROM story_entities WHERE story_id = ?1 ORDER BY entity_id",
        )?,
        affected,
        eligible_at: from_text(eligible.as_deref().unwrap_or(&updated))?,
    }))
}

fn candidates(
    conn: &Connection,
    story: i64,
    facts: &StoryFacts,
    now: &str,
) -> Result<Vec<Candidate>, StorageError> {
    let components: Vec<String> = facts
        .affected
        .iter()
        .filter_map(|a| a.component.purl.clone().or_else(|| a.component.cpe.clone()))
        .collect();
    let mut entries: BTreeMap<i64, Vec<(String, Option<String>)>> = BTreeMap::new();
    if !components.is_empty() {
        let sql = format!(
            "SELECT i.subscriber_id, i.component, i.version FROM inventory_entries i \
             JOIN subscribers s ON s.id = i.subscriber_id \
             WHERE s.active = 1 AND i.component IN ({})",
            placeholders(components.len())
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(&components), |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        for row in rows {
            let (subscriber, component, version) = row?;
            entries
                .entry(subscriber)
                .or_default()
                .push((component, (!version.is_empty()).then_some(version)));
        }
    }
    let mut watched: BTreeMap<i64, Vec<String>> = BTreeMap::new();
    if !facts.entities.is_empty() {
        let sql = format!(
            "SELECT w.subscriber_id, w.entity_id FROM watches w \
             JOIN subscribers s ON s.id = w.subscriber_id \
             WHERE s.active = 1 AND w.entity_id IN ({})",
            placeholders(facts.entities.len())
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(&facts.entities), |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (subscriber, entity) = row?;
            watched.entry(subscriber).or_default().push(entity);
        }
    }
    let subscribers: Vec<i64> = entries.keys().chain(watched.keys()).copied().collect();
    let subscribers = secure_notify_core::matching::distinct(&subscribers);
    let mut candidates = Vec::with_capacity(subscribers.len());
    for subscriber in subscribers {
        let min_severity: Option<String> = conn.query_row(
            "SELECT min_severity FROM subscribers WHERE id = ?1",
            [subscriber],
            |r| r.get(0),
        )?;
        let mut rules = Vec::new();
        let mut stmt = conn.prepare(
            "SELECT match, action, min_severity FROM rules WHERE subscriber_id = ?1 ORDER BY position",
        )?;
        let rows = stmt.query_map([subscriber], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })?;
        for row in rows {
            let (conditions, action, min) = row?;
            rules.push(Rule {
                conditions: serde_json::from_str::<RuleMatch>(&conditions).map_err(corrupt)?,
                action: if action == "ignore" {
                    RuleAction::Ignore
                } else {
                    RuleAction::Notify
                },
                min_severity: min.as_deref().and_then(SeverityLevel::parse),
            });
        }
        let mut mutes = Vec::new();
        let mut stmt = conn.prepare(
            "SELECT scope, value FROM mutes WHERE subscriber_id = ?1 AND (until IS NULL OR until > ?2)",
        )?;
        let rows = stmt.query_map(params![subscriber, now], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (scope, value) = row?;
            let mute = match scope.as_str() {
                "story" => value.parse().ok().map(|id| Mute::Story(StoryId(id))),
                "entity" => Some(Mute::Entity(value)),
                "component" => Some(Mute::Component(value)),
                _ => value
                    .rsplit_once('@')
                    .map(|(c, v)| Mute::ComponentVersion(c.to_string(), v.to_string())),
            };
            mutes.extend(mute);
        }
        let delivered: Option<String> = conn
            .query_row(
                "SELECT snapshot FROM deliveries WHERE subscriber_id = ?1 AND story_id = ?2 \
                 ORDER BY id DESC LIMIT 1",
                params![subscriber, story],
                |r| r.get(0),
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                e => Err(e),
            })?;
        candidates.push(Candidate {
            subscriber: SubscriberId(subscriber),
            entries: entries.remove(&subscriber).unwrap_or_default(),
            watched: watched.remove(&subscriber).unwrap_or_default(),
            rules,
            min_severity: min_severity.as_deref().and_then(SeverityLevel::parse),
            mutes,
            delivered: delivered
                .map(|s| serde_json::from_str::<Snapshot>(&s))
                .transpose()
                .map_err(corrupt)?,
        });
    }
    Ok(candidates)
}

impl MatchStore for Database {
    async fn stories_to_match(
        &self,
        now: jiff::Timestamp,
        limit: usize,
    ) -> Result<Vec<StoryId>, StoreError> {
        let now = to_text(now);
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        Ok(self
            .read(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id FROM stories WHERE needs_match = 1 \
                     AND (eligible_at IS NULL OR eligible_at <= ?1) ORDER BY eligible_at LIMIT ?2",
                )?;
                let rows = stmt.query_map(params![now, limit], |r| Ok(StoryId(r.get(0)?)))?;
                Ok(rows.collect::<Result<_, _>>()?)
            })
            .await?)
    }

    async fn match_input(
        &self,
        story: StoryId,
        now: jiff::Timestamp,
    ) -> Result<Option<(StoryFacts, Vec<Candidate>)>, StoreError> {
        let now = to_text(now);
        Ok(self
            .read(move |conn| {
                let Some(facts) = story_facts(conn, story.0)? else {
                    return Ok(None);
                };
                let candidates = candidates(conn, story.0, &facts, &now)?;
                Ok(Some((facts, candidates)))
            })
            .await?)
    }

    async fn enqueue(
        &self,
        story: StoryId,
        items: Vec<Enqueue>,
        now: jiff::Timestamp,
    ) -> Result<(), StoreError> {
        Ok(self
            .write(move |tx| {
                let now = to_text(now);
                for item in &items {
                    let reason = serde_json::to_string(&item.reason).map_err(corrupt)?;
                    tx.execute(
                        "INSERT INTO notification_queue \
                         (subscriber_id, story_id, kind, reason, due_at, created_at) \
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
                         ON CONFLICT (subscriber_id, story_id, kind) \
                         DO UPDATE SET reason = excluded.reason, due_at = excluded.due_at",
                        params![
                            item.subscriber.0,
                            story.0,
                            item.kind.as_str(),
                            reason,
                            to_text(item.due_at),
                            now
                        ],
                    )?;
                }
                tx.execute(
                    "UPDATE stories SET needs_match = 0 WHERE id = ?1",
                    [story.0],
                )?;
                Ok(())
            })
            .await?)
    }
}

impl secure_notify_core::rules::RuleStore for Database {
    async fn custom_rules(&self, subscriber: SubscriberId) -> Result<Vec<Rule>, StoreError> {
        Ok(self
            .read(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT match, action, min_severity FROM rules WHERE subscriber_id = ?1 ORDER BY position",
                )?;
                let rows = stmt.query_map([subscriber.0], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<String>>(2)?))
                })?;
                let mut rules = Vec::new();
                for row in rows {
                    let (conditions, action, min) = row?;
                    rules.push(Rule {
                        conditions: serde_json::from_str(&conditions).map_err(corrupt)?,
                        action: if action == "ignore" { RuleAction::Ignore } else { RuleAction::Notify },
                        min_severity: min.as_deref().and_then(SeverityLevel::parse),
                    });
                }
                Ok(rules)
            })
            .await?)
    }

    async fn reset_rules(&self, subscriber: SubscriberId) -> Result<(), StoreError> {
        Ok(self
            .write(move |tx| {
                tx.execute("DELETE FROM rules WHERE subscriber_id = ?1", [subscriber.0])?;
                Ok(())
            })
            .await?)
    }

    async fn min_severity(
        &self,
        subscriber: SubscriberId,
    ) -> Result<Option<SeverityLevel>, StoreError> {
        Ok(self
            .read(move |conn| {
                let level: Option<String> = conn.query_row(
                    "SELECT min_severity FROM subscribers WHERE id = ?1",
                    [subscriber.0],
                    |r| r.get(0),
                )?;
                Ok(level.as_deref().and_then(SeverityLevel::parse))
            })
            .await?)
    }

    async fn set_min_severity(
        &self,
        subscriber: SubscriberId,
        level: Option<SeverityLevel>,
    ) -> Result<(), StoreError> {
        Ok(self
            .write(move |tx| {
                tx.execute(
                    "UPDATE subscribers SET min_severity = ?1 WHERE id = ?2",
                    params![level.map(SeverityLevel::as_str), subscriber.0],
                )?;
                Ok(())
            })
            .await?)
    }
}
