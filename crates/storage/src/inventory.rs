// SPDX-License-Identifier: AGPL-3.0-or-later

//! Inventory entries: flat component and version pairs; an unknown version is stored as `''`.

use rusqlite::params;
use secure_notify_core::inventory::{InventoryEntry, InventoryStore, MergeCounts, ParsedComponent};
use secure_notify_core::ports::StoreError;
use secure_notify_core::subscribers::SubscriberId;

use crate::Database;
use crate::time::{from_text, to_text};

impl InventoryStore for Database {
    async fn merge_inventory(
        &self,
        subscriber: SubscriberId,
        entries: Vec<ParsedComponent>,
        now: jiff::Timestamp,
    ) -> Result<MergeCounts, StoreError> {
        Ok(self
            .write(move |tx| {
                let now = to_text(now);
                let mut insert = tx.prepare(
                    "INSERT OR IGNORE INTO inventory_entries \
                     (subscriber_id, component, version, last_seen_at) VALUES (?1, ?2, ?3, ?4)",
                )?;
                let mut refresh = tx.prepare(
                    "UPDATE inventory_entries SET last_seen_at = ?4 \
                     WHERE subscriber_id = ?1 AND component = ?2 AND version = ?3",
                )?;
                let mut counts = MergeCounts::default();
                for entry in &entries {
                    let version = entry.version.as_deref().unwrap_or_default();
                    let values = params![subscriber.0, entry.purl, version, now];
                    if insert.execute(values)? > 0 {
                        counts.added += 1;
                    } else {
                        refresh.execute(values)?;
                        counts.refreshed += 1;
                    }
                }
                Ok(counts)
            })
            .await?)
    }

    async fn inventory(&self, subscriber: SubscriberId) -> Result<Vec<InventoryEntry>, StoreError> {
        Ok(self
            .read(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT component, version, last_seen_at FROM inventory_entries \
                     WHERE subscriber_id = ?1 ORDER BY component, version",
                )?;
                let rows = stmt.query_map([subscriber.0], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                })?;
                let mut entries = Vec::new();
                for row in rows {
                    let (component, version, seen) = row?;
                    entries.push(InventoryEntry {
                        component,
                        version: (!version.is_empty()).then_some(version),
                        last_seen_at: from_text(&seen)?,
                    });
                }
                Ok(entries)
            })
            .await?)
    }

    async fn remove_inventory(
        &self,
        subscriber: SubscriberId,
        component: &str,
        version: Option<Option<String>>,
    ) -> Result<usize, StoreError> {
        let component = component.to_string();
        Ok(self
            .write(move |tx| {
                Ok(match version {
                    None => tx.execute(
                        "DELETE FROM inventory_entries WHERE subscriber_id = ?1 AND component = ?2",
                        params![subscriber.0, component],
                    )?,
                    Some(version) => tx.execute(
                        "DELETE FROM inventory_entries \
                         WHERE subscriber_id = ?1 AND component = ?2 AND version = ?3",
                        params![subscriber.0, component, version.unwrap_or_default()],
                    )?,
                })
            })
            .await?)
    }

    async fn clear_inventory(&self, subscriber: SubscriberId) -> Result<usize, StoreError> {
        Ok(self
            .write(move |tx| {
                Ok(tx.execute(
                    "DELETE FROM inventory_entries WHERE subscriber_id = ?1",
                    [subscriber.0],
                )?)
            })
            .await?)
    }
}
