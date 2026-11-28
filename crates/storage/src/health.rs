// SPDX-License-Identifier: AGPL-3.0-or-later

//! Instance counters and admin recipients (docs/operations/self-hosting.md#operations).

use secure_notify_core::health::{HealthStore, InstanceStats};
use secure_notify_core::ports::StoreError;
use secure_notify_core::subscribers::EndpointRef;

use crate::Database;
use crate::delivery::recipient;

impl HealthStore for Database {
    async fn instance_stats(&self) -> Result<InstanceStats, StoreError> {
        Ok(self
            .read(|conn| {
                let count = |sql: &str| -> Result<u64, rusqlite::Error> {
                    let n: i64 = conn.query_row(sql, [], |r| r.get(0))?;
                    Ok(u64::try_from(n).unwrap_or(0))
                };
                Ok(InstanceStats {
                    subscribers: count("SELECT count(*) FROM subscribers WHERE active = 1")?,
                    stories: count("SELECT count(*) FROM stories")?,
                    events: count("SELECT count(*) FROM events")?,
                    queued: count("SELECT count(*) FROM notification_queue")?,
                    pending_scans: count("SELECT count(*) FROM pending_scans")?,
                })
            })
            .await?)
    }

    async fn admin_recipients(&self) -> Result<Vec<(EndpointRef, String)>, StoreError> {
        Ok(self
            .read(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT id FROM subscribers WHERE role = 'admin' AND active = 1 ORDER BY id",
                )?;
                let admins: Vec<i64> = stmt
                    .query_map([], |r| r.get(0))?
                    .collect::<Result<_, _>>()?;
                let mut recipients = Vec::new();
                for admin in admins {
                    if let (Some((_, endpoint)), language) = recipient(conn, admin)? {
                        recipients.push((endpoint, language));
                    }
                }
                Ok(recipients)
            })
            .await?)
    }
}
