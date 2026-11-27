// SPDX-License-Identifier: AGPL-3.0-or-later

//! Watched entities.

use rusqlite::{OptionalExtension, params};
use secure_notify_core::ports::StoreError;
use secure_notify_core::subscribers::SubscriberId;
use secure_notify_core::watches::{WatchOrigin, WatchStore};

use crate::time::to_text;
use crate::{Database, StorageError};

fn origin(text: &str) -> Result<WatchOrigin, StorageError> {
    match text {
        "manual" => Ok(WatchOrigin::Manual),
        "derived" => Ok(WatchOrigin::Derived),
        other => Err(StorageError::Corrupt(format!("watch origin `{other}`"))),
    }
}

impl WatchStore for Database {
    async fn add_watch(
        &self,
        subscriber: SubscriberId,
        entity: &str,
        new_origin: WatchOrigin,
        now: jiff::Timestamp,
    ) -> Result<bool, StoreError> {
        let entity = entity.to_string();
        Ok(self
            .write(move |tx| {
                let existing: Option<String> = tx
                    .query_row(
                        "SELECT origin FROM watches WHERE subscriber_id = ?1 AND entity_id = ?2",
                        params![subscriber.0, entity],
                        |r| r.get(0),
                    )
                    .optional()?;
                let existing = existing.as_deref().map(origin).transpose()?;
                match (existing, new_origin) {
                    (Some(WatchOrigin::Manual), _) => Ok(true),
                    (Some(WatchOrigin::Derived), WatchOrigin::Derived) => Ok(false),
                    (Some(WatchOrigin::Derived), WatchOrigin::Manual) => {
                        tx.execute(
                            "UPDATE watches SET origin = 'manual' \
                             WHERE subscriber_id = ?1 AND entity_id = ?2",
                            params![subscriber.0, entity],
                        )?;
                        Ok(false)
                    }
                    (None, _) => {
                        tx.execute(
                            "INSERT INTO watches (subscriber_id, entity_id, origin, created_at) \
                             VALUES (?1, ?2, ?3, ?4)",
                            params![subscriber.0, entity, new_origin.as_str(), to_text(now)],
                        )?;
                        Ok(false)
                    }
                }
            })
            .await?)
    }

    async fn remove_watch(
        &self,
        subscriber: SubscriberId,
        entity: &str,
    ) -> Result<bool, StoreError> {
        let entity = entity.to_string();
        Ok(self
            .write(move |tx| {
                Ok(tx.execute(
                    "DELETE FROM watches WHERE subscriber_id = ?1 AND entity_id = ?2",
                    params![subscriber.0, entity],
                )? > 0)
            })
            .await?)
    }

    async fn watches(
        &self,
        subscriber: SubscriberId,
    ) -> Result<Vec<(String, WatchOrigin)>, StoreError> {
        Ok(self
            .read(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT entity_id, origin FROM watches WHERE subscriber_id = ?1 \
                     ORDER BY entity_id",
                )?;
                let rows = stmt.query_map([subscriber.0], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
                })?;
                let mut list = Vec::new();
                for row in rows {
                    let (entity, text) = row?;
                    list.push((entity, origin(&text)?));
                }
                Ok(list)
            })
            .await?)
    }
}
