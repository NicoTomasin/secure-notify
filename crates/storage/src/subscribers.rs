// SPDX-License-Identifier: AGPL-3.0-or-later

//! Subscribers, endpoints, pairing, and invites.

use rusqlite::{OptionalExtension, Transaction, params};
use secure_notify_core::ports::StoreError;
use secure_notify_core::subscribers::{
    EndpointRef, Joined, PAIRING_CODE_KEY, Role, Subscriber, SubscriberId, SubscriberStore,
    SubscriberSummary,
};

use crate::time::{from_text, to_text};
use crate::{Database, StorageError};

fn role(text: &str) -> Result<Role, StorageError> {
    match text {
        "admin" => Ok(Role::Admin),
        "member" => Ok(Role::Member),
        other => Err(StorageError::Corrupt(format!("role `{other}`"))),
    }
}

fn subscriber_from_row(
    id: i64,
    role_text: &str,
    language: String,
    created_at: &str,
) -> Result<Subscriber, StorageError> {
    Ok(Subscriber {
        id: SubscriberId(id),
        role: role(role_text)?,
        language,
        created_at: from_text(created_at)?,
    })
}

/// Creates a person with a primary private endpoint.
fn create_person(
    tx: &Transaction<'_>,
    role: Role,
    endpoint: &EndpointRef,
    language: &str,
    now: &str,
) -> Result<SubscriberId, StorageError> {
    tx.execute(
        "INSERT INTO subscribers (kind, role, language, created_at) VALUES ('person', ?1, ?2, ?3)",
        params![role.as_str(), language, now],
    )?;
    let id = tx.last_insert_rowid();
    tx.execute(
        "INSERT INTO endpoints (subscriber_id, platform, address, kind, is_primary, created_at) \
         VALUES (?1, ?2, ?3, 'private', 1, ?4)",
        params![id, endpoint.platform, endpoint.address, now],
    )?;
    Ok(SubscriberId(id))
}

fn joined(id: SubscriberId, role: Role, language: &str, now: jiff::Timestamp) -> Joined {
    Joined::Joined(Subscriber {
        id,
        role,
        language: language.to_string(),
        created_at: now,
    })
}

impl SubscriberStore for Database {
    async fn subscriber_by_endpoint(
        &self,
        endpoint: &EndpointRef,
    ) -> Result<Option<Subscriber>, StoreError> {
        let endpoint = endpoint.clone();
        Ok(self
            .read(move |conn| {
                let row = conn
                    .query_row(
                        "SELECT s.id, s.role, s.language, s.created_at FROM endpoints e \
                         JOIN subscribers s ON s.id = e.subscriber_id \
                         WHERE e.platform = ?1 AND e.address = ?2 AND e.active = 1 \
                           AND s.active = 1 AND s.kind = 'person'",
                        params![endpoint.platform, endpoint.address],
                        |r| {
                            Ok((
                                r.get::<_, i64>(0)?,
                                r.get::<_, String>(1)?,
                                r.get::<_, String>(2)?,
                                r.get::<_, String>(3)?,
                            ))
                        },
                    )
                    .optional()?;
                row.map(|(id, role, language, created)| {
                    subscriber_from_row(id, &role, language, &created)
                })
                .transpose()
            })
            .await?)
    }

    async fn admin_exists(&self) -> Result<bool, StoreError> {
        Ok(self
            .read(|conn| {
                Ok(conn.query_row(
                    "SELECT EXISTS (SELECT 1 FROM subscribers WHERE role = 'admin')",
                    [],
                    |r| r.get(0),
                )?)
            })
            .await?)
    }

    async fn pair(
        &self,
        code: &str,
        endpoint: &EndpointRef,
        language: &str,
        now: jiff::Timestamp,
    ) -> Result<Joined, StoreError> {
        let (code, endpoint, language) = (code.to_string(), endpoint.clone(), language.to_string());
        Ok(self
            .write(move |tx| {
                let stored: Option<String> = tx
                    .query_row(
                        "SELECT value FROM instance_state WHERE key = ?1",
                        [PAIRING_CODE_KEY],
                        |r| r.get(0),
                    )
                    .optional()?;
                let admin_exists: bool = tx.query_row(
                    "SELECT EXISTS (SELECT 1 FROM subscribers WHERE role = 'admin')",
                    [],
                    |r| r.get(0),
                )?;
                if admin_exists || stored.as_deref() != Some(code.as_str()) {
                    return Ok(Joined::InvalidCode);
                }
                let id = create_person(tx, Role::Admin, &endpoint, &language, &to_text(now))?;
                tx.execute(
                    "DELETE FROM instance_state WHERE key = ?1",
                    [PAIRING_CODE_KEY],
                )?;
                Ok(joined(id, Role::Admin, &language, now))
            })
            .await?)
    }

    async fn create_invite(
        &self,
        code: &str,
        created_by: SubscriberId,
        now: jiff::Timestamp,
        expires_at: jiff::Timestamp,
    ) -> Result<(), StoreError> {
        let code = code.to_string();
        Ok(self
            .write(move |tx| {
                tx.execute(
                    "INSERT INTO invites (code, created_by, kind, created_at, expires_at) \
                     VALUES (?1, ?2, 'person', ?3, ?4)",
                    params![code, created_by.0, to_text(now), to_text(expires_at)],
                )?;
                Ok(())
            })
            .await?)
    }

    async fn redeem_invite(
        &self,
        code: &str,
        endpoint: &EndpointRef,
        language: &str,
        now: jiff::Timestamp,
    ) -> Result<Joined, StoreError> {
        let (code, endpoint, language) = (code.to_string(), endpoint.clone(), language.to_string());
        Ok(self
            .write(move |tx| {
                let now_text = to_text(now);
                let valid: bool = tx.query_row(
                    "SELECT EXISTS (SELECT 1 FROM invites WHERE code = ?1 AND kind = 'person' \
                     AND used_by IS NULL AND used_at IS NULL AND expires_at > ?2)",
                    params![code, now_text],
                    |r| r.get(0),
                )?;
                if !valid {
                    return Ok(Joined::InvalidCode);
                }
                let id = create_person(tx, Role::Member, &endpoint, &language, &now_text)?;
                tx.execute(
                    "UPDATE invites SET used_by = ?1, used_at = ?2 WHERE code = ?3",
                    params![id.0, now_text, code],
                )?;
                Ok(joined(id, Role::Member, &language, now))
            })
            .await?)
    }

    async fn list_subscribers(&self) -> Result<Vec<SubscriberSummary>, StoreError> {
        Ok(self
            .read(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT s.id, s.role, s.language, s.created_at, s.active, \
                     (SELECT group_concat(platform, ',') FROM \
                        (SELECT DISTINCT platform FROM endpoints e \
                         WHERE e.subscriber_id = s.id AND e.active = 1 ORDER BY platform)) \
                     FROM subscribers s WHERE s.kind = 'person' ORDER BY s.id",
                )?;
                let rows = stmt.query_map([], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, bool>(4)?,
                        r.get::<_, Option<String>>(5)?,
                    ))
                })?;
                let mut list = Vec::new();
                for row in rows {
                    let (id, role, language, created, active, platforms) = row?;
                    list.push(SubscriberSummary {
                        subscriber: subscriber_from_row(id, &role, language, &created)?,
                        platforms: platforms
                            .map_or_else(Vec::new, |p| p.split(',').map(str::to_string).collect()),
                        active,
                    });
                }
                Ok(list)
            })
            .await?)
    }

    async fn remove_subscriber(&self, id: SubscriberId) -> Result<bool, StoreError> {
        Ok(self
            .write(move |tx| Ok(tx.execute("DELETE FROM subscribers WHERE id = ?1", [id.0])? > 0))
            .await?)
    }

    async fn set_language(&self, id: SubscriberId, language: &str) -> Result<(), StoreError> {
        let language = language.to_string();
        Ok(self
            .write(move |tx| {
                tx.execute(
                    "UPDATE subscribers SET language = ?1 WHERE id = ?2",
                    params![language, id.0],
                )?;
                Ok(())
            })
            .await?)
    }
}
