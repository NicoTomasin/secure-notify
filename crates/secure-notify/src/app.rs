// SPDX-License-Identifier: AGPL-3.0-or-later

//! Wiring of the core to its adapters.

use std::sync::Arc;

use secure_notify_conversation::{About, Conversation, I18n};
use secure_notify_core::app::Core;
use secure_notify_core::ports::{Clock, Random};
use secure_notify_storage::Database;

use crate::config::Settings;

/// Random bytes from the operating system's secure generator.
pub(crate) struct OsRandom;

impl Random for OsRandom {
    fn fill(&self, buf: &mut [u8]) {
        // Codes must never come from a weak source; without the OS generator the instance cannot
        // run safely, so this is treated as fatal.
        if let Err(error) = getrandom::fill(buf) {
            panic!("the operating system's random generator failed: {error}");
        }
    }
}

/// Builds the conversation layer over the database.
pub(crate) fn conversation(
    settings: &Settings,
    database: Database,
    clock: Arc<dyn Clock>,
) -> Result<Conversation<Database>, String> {
    let i18n = I18n::load().map_err(|e| format!("cannot load translations: {e}"))?;
    let taxonomy = crate::content::embedded_taxonomy()?;
    let core = Core::new(database, Arc::new(taxonomy), clock, Arc::new(OsRandom));
    let about = About {
        version: env!("CARGO_PKG_VERSION").to_string(),
        source_url: settings.source_url.clone(),
    };
    Ok(Conversation::new(core, i18n, about))
}

/// While no admin exists, makes sure a pairing code exists and prints it to the logs
/// (docs/operations/self-hosting.md#first-run).
pub(crate) async fn announce_pairing_code(
    conversation: &Conversation<Database>,
) -> Result<(), String> {
    let code = conversation
        .subscribers()
        .ensure_pairing_code()
        .await
        .map_err(|e| format!("cannot prepare the pairing code: {e}"))?;
    if let Some(code) = code {
        tracing::info!(
            pairing_code = %code,
            "no admin yet: send /start <pairing_code> to the bot to become the admin"
        );
    }
    Ok(())
}
