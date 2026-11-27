// SPDX-License-Identifier: AGPL-3.0-or-later

//! Fluent translations embedded from `locales/`, with English as the reference and fallback
//! (docs/architecture/delivery.md#localization-i18n).

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource};
use unic_langid::LanguageIdentifier;

/// The reference locale; missing keys fall back to it.
pub const REFERENCE_LANGUAGE: &str = "en";

/// Embedded Fluent sources per language.
const SOURCES: &[(&str, &[&str])] = include!(concat!(env!("OUT_DIR"), "/locales.rs"));

/// A broken translation file.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct LocaleError(pub String);

/// All translations.
pub struct I18n {
    bundles: Vec<(String, FluentBundle<FluentResource>)>,
}

impl I18n {
    /// Loads the embedded translations.
    pub fn load() -> Result<Self, LocaleError> {
        let mut bundles = Vec::new();
        for (language, files) in SOURCES {
            let id: LanguageIdentifier = language
                .parse()
                .map_err(|e| LocaleError(format!("locales/{language}: {e}")))?;
            let mut bundle = FluentBundle::new_concurrent(vec![id]);
            // Chat platforms show the Unicode isolation marks literally in some clients.
            bundle.set_use_isolating(false);
            for source in *files {
                let resource =
                    FluentResource::try_new((*source).to_string()).map_err(|(_, e)| {
                        LocaleError(format!("locales/{language}: syntax errors: {e:?}"))
                    })?;
                bundle.add_resource(resource).map_err(|e| {
                    LocaleError(format!("locales/{language}: duplicate messages: {e:?}"))
                })?;
            }
            bundles.push(((*language).to_string(), bundle));
        }
        if !bundles.iter().any(|(l, _)| l == REFERENCE_LANGUAGE) {
            return Err(LocaleError("locales/en is missing".to_string()));
        }
        Ok(Self { bundles })
    }

    /// Available language codes, sorted.
    pub fn languages(&self) -> Vec<&str> {
        self.bundles.iter().map(|(l, _)| l.as_str()).collect()
    }

    /// The supported language matching a client's reported language (`es-AR` → `es`), if any.
    pub fn match_language(&self, reported: &str) -> Option<&str> {
        let primary = reported
            .split(['-', '_'])
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        self.languages().into_iter().find(|l| *l == primary)
    }

    fn bundle(&self, language: &str) -> Option<&FluentBundle<FluentResource>> {
        self.bundles
            .iter()
            .find(|(l, _)| l == language)
            .map(|(_, b)| b)
    }

    /// The message `key` in `language`, falling back to English, then to the key itself.
    pub fn text(&self, language: &str, key: &str, args: Option<&FluentArgs<'_>>) -> String {
        for candidate in [language, REFERENCE_LANGUAGE] {
            let Some(bundle) = self.bundle(candidate) else {
                continue;
            };
            let Some(pattern) = bundle.get_message(key).and_then(|m| m.value()) else {
                continue;
            };
            let mut errors = Vec::new();
            let text = bundle.format_pattern(pattern, args, &mut errors);
            if errors.is_empty() {
                return text.into_owned();
            }
            tracing::warn!(language = candidate, key, "translation failed to format");
        }
        tracing::warn!(key, "missing translation");
        key.to_string()
    }

    /// Message IDs defined for `language`.
    pub fn keys(&self, language: &str) -> Vec<String> {
        SOURCES
            .iter()
            .filter(|(l, _)| *l == language)
            .flat_map(|(_, files)| files.iter())
            .filter_map(|source| FluentResource::try_new((*source).to_string()).ok())
            .flat_map(|resource| {
                resource
                    .entries()
                    .filter_map(|entry| match entry {
                        fluent_syntax::ast::Entry::Message(m) => Some(m.id.name.to_string()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }
}
