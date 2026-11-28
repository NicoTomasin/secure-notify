// SPDX-License-Identifier: AGPL-3.0-or-later

//! Commands, permissions, the platform-neutral message model, and i18n for secure-notify
//! (docs/architecture/platforms.md#conversation-layer).
//!
//! Commands are defined once here; platform plugins render the message model natively and pass
//! incoming messages to [`Conversation::handle`].

pub mod commands;
mod handler;
pub mod i18n;
pub mod message;
mod notice;
mod status;

pub use handler::{ATTRIBUTION, About, Attachment, ChatKind, Conversation, Incoming, LICENSE};
pub use i18n::I18n;
pub use message::{Action, Block, CodeKind, GeneratedCode, Message, render_plain};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translations_only_define_keys_of_the_reference_locale() {
        // delivery.md: English is the reference; other locales translate its keys.
        let i18n = I18n::load().unwrap();
        let reference = i18n.keys("en");
        for language in i18n.languages() {
            for key in i18n.keys(language) {
                assert!(
                    reference.contains(&key),
                    "locales/{language} defines unknown key {key}"
                );
            }
        }
    }

    #[test]
    fn every_command_has_a_reference_description() {
        let i18n = I18n::load().unwrap();
        let reference = i18n.keys("en");
        for spec in commands::COMMANDS {
            assert!(
                reference.iter().any(|k| k == spec.description),
                "{} has no description in locales/en",
                spec.name
            );
        }
    }

    #[test]
    fn missing_keys_fall_back_to_english() {
        let i18n = I18n::load().unwrap();
        // `language-name` exists everywhere; an unknown language falls back to English.
        assert_eq!(i18n.text("xx", "language-name", None), "English");
        assert_eq!(i18n.text("es", "language-name", None), "Español");
    }
}
