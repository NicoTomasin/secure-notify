// SPDX-License-Identifier: AGPL-3.0-or-later

//! Instance-wide key/value state.

use rusqlite::{OptionalExtension, params};
use secure_notify_core::ports::{InstanceStore, StoreError};

use crate::{Database, StorageError};

impl From<StorageError> for StoreError {
    fn from(error: StorageError) -> Self {
        Self(error.to_string())
    }
}

impl InstanceStore for Database {
    async fn instance_value(&self, key: &str) -> Result<Option<String>, StoreError> {
        let key = key.to_string();
        Ok(self
            .read(move |conn| {
                Ok(conn
                    .query_row(
                        "SELECT value FROM instance_state WHERE key = ?1",
                        [key],
                        |row| row.get(0),
                    )
                    .optional()?)
            })
            .await?)
    }

    async fn set_instance_value(&self, key: &str, value: &str) -> Result<(), StoreError> {
        let (key, value) = (key.to_string(), value.to_string());
        Ok(self
            .write(move |tx| {
                tx.execute(
                    "INSERT INTO instance_state (key, value) VALUES (?1, ?2) \
                     ON CONFLICT (key) DO UPDATE SET value = excluded.value",
                    params![key, value],
                )?;
                Ok(())
            })
            .await?)
    }
}
