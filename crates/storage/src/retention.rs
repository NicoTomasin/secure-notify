// SPDX-License-Identifier: AGPL-3.0-or-later

//! Retention: expiring payloads, compacting stories, deleting old deduplication records.

use rusqlite::params;
use secure_notify_core::ports::StoreError;
use secure_notify_core::retention::{RetentionCounts, RetentionStore};

use crate::Database;
use crate::time::to_text;

impl RetentionStore for Database {
    async fn compact(
        &self,
        details_before: jiff::Timestamp,
        records_before: jiff::Timestamp,
    ) -> Result<RetentionCounts, StoreError> {
        let (details, records) = (to_text(details_before), to_text(records_before));
        Ok(self
            .write(move |tx| {
                let payloads_expired = tx.execute(
                    "UPDATE events SET payload = NULL \
                     WHERE mirror = 0 AND payload IS NOT NULL AND updated_at < ?1",
                    [&details],
                )?;
                tx.execute(
                    "CREATE TEMP TABLE IF NOT EXISTS compacting (id INTEGER PRIMARY KEY)",
                    [],
                )?;
                tx.execute("DELETE FROM compacting", [])?;
                tx.execute(
                    "INSERT INTO compacting (id) SELECT s.id FROM stories s \
                     WHERE s.compacted = 0 AND s.updated_at < ?1 \
                     AND NOT EXISTS (SELECT 1 FROM events e WHERE e.story_id = s.id AND e.mirror = 1)",
                    [&details],
                )?;
                for sql in [
                    "DELETE FROM story_affected WHERE story_id IN (SELECT id FROM compacting)",
                    "DELETE FROM story_entities WHERE story_id IN (SELECT id FROM compacting)",
                    "DELETE FROM story_relations WHERE story_id IN (SELECT id FROM compacting) \
                     OR related_story_id IN (SELECT id FROM compacting)",
                ] {
                    tx.execute(sql, [])?;
                }
                let stories_compacted = tx.execute(
                    "UPDATE stories SET compacted = 1, summary = NULL \
                     WHERE id IN (SELECT id FROM compacting)",
                    [],
                )?;
                tx.execute("DELETE FROM compacting", [])?;
                let stories_deleted = tx.execute(
                    "DELETE FROM stories WHERE compacted = 1 AND updated_at < ?1",
                    params![records],
                )?;
                Ok(RetentionCounts {
                    payloads_expired,
                    stories_compacted,
                    stories_deleted,
                })
            })
            .await?)
    }
}
