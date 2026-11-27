// SPDX-License-Identifier: AGPL-3.0-or-later

//! Source state: cursors, backfill status, and health.

use rusqlite::params;
use secure_notify_core::ports::StoreError;
use secure_notify_core::sources::{SourceState, SourceStateStore};

use crate::Database;

impl SourceStateStore for Database {
    async fn source_state(&self, source: &str) -> Result<SourceState, StoreError> {
        let source = source.to_string();
        Ok(self
            .read(move |conn| {
                let mut stmt =
                    conn.prepare("SELECT key, value FROM source_state WHERE source_id = ?1")?;
                let rows = stmt.query_map([source], |r| Ok((r.get(0)?, r.get(1)?)))?;
                let mut state = SourceState::new();
                for row in rows {
                    let (key, value) = row?;
                    state.insert(key, value);
                }
                Ok(state)
            })
            .await?)
    }

    async fn save_source_state(&self, source: &str, state: &SourceState) -> Result<(), StoreError> {
        let (source, state) = (source.to_string(), state.clone());
        Ok(self
            .write(move |tx| {
                tx.execute("DELETE FROM source_state WHERE source_id = ?1", [&source])?;
                let mut insert = tx.prepare(
                    "INSERT INTO source_state (source_id, key, value) VALUES (?1, ?2, ?3)",
                )?;
                for (key, value) in &state {
                    insert.execute(params![source, key, value])?;
                }
                Ok(())
            })
            .await?)
    }
}
