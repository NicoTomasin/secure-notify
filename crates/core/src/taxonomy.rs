// SPDX-License-Identifier: AGPL-3.0-or-later

//! The taxonomy and deterministic tagging (docs/architecture/taxonomy.md#tagging).

use std::collections::BTreeSet;

use secure_notify_model::{NormalizedEvent, TaxonomyEntity, load_taxonomy, normalize_text};

/// The loaded taxonomy.
#[derive(Debug, Clone, Default)]
pub struct Taxonomy {
    entities: Vec<TaxonomyEntity>,
}

impl Taxonomy {
    /// Loads taxonomy files, given as paths relative to `content/taxonomy/` and their text.
    pub fn load(files: &[(String, String)]) -> Result<Self, Vec<String>> {
        Ok(Self {
            entities: load_taxonomy(files)?,
        })
    }

    /// Every entity, sorted by ID.
    pub fn entities(&self) -> impl Iterator<Item = &TaxonomyEntity> {
        self.entities.iter()
    }

    /// The entity with this ID.
    pub fn get(&self, id: &str) -> Option<&TaxonomyEntity> {
        self.entities.iter().find(|e| e.id == id)
    }

    /// The entity a subscriber means: an ID, the name, or an alias, in any letter case.
    pub fn resolve(&self, query: &str) -> Option<&TaxonomyEntity> {
        let query = query.trim().to_lowercase();
        self.entities.iter().find(|e| {
            e.id == query
                || e.name.to_lowercase() == query
                || e.aliases.iter().any(|a| a.to_lowercase() == query)
        })
    }

    /// Entities mapped exactly to a purl (without version).
    pub fn entities_for_purl(&self, purl: &str) -> impl Iterator<Item = &str> {
        self.entities
            .iter()
            .filter(move |e| e.mappings.purl.iter().any(|p| p == purl))
            .map(|e| e.id.as_str())
    }

    /// Entities the core adds to an event: exact purl mappings of its affected components and
    /// keyword matches in its title and summary. Deterministic for the same input and taxonomy.
    /// Entities the source already set are not repeated.
    pub fn tag(&self, event: &NormalizedEvent) -> BTreeSet<String> {
        let mut tags = BTreeSet::new();
        for affected in &event.affected {
            if let Some(purl) = &affected.component.purl {
                tags.extend(self.entities_for_purl(purl).map(str::to_string));
            }
        }
        let text = normalize_text(&format!(
            "{} {}",
            event.title,
            event.summary.as_deref().unwrap_or_default()
        ));
        for entity in &self.entities {
            let keyword_found = entity.keywords.iter().any(|k| contains_word(&text, k));
            let qualified = entity.requires_any.is_empty()
                || entity.requires_any.iter().any(|q| contains_word(&text, q));
            if keyword_found && qualified {
                tags.insert(entity.id.clone());
            }
        }
        for own in event.entities.iter() {
            tags.remove(own);
        }
        tags
    }
}

/// Whether `needle` occurs in `text` with word boundaries (no letter or digit on either side).
fn contains_word(text: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    text.match_indices(needle).any(|(start, _)| {
        let before = text[..start].chars().next_back();
        let after = text[start + needle.len()..].chars().next();
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ambiguous_keywords_need_a_qualifier() {
        // taxonomy.md#tagging: the documented `go` example; synthetic entity and texts.
        let files = [(
            "technologies/go.toml".to_string(),
            "id = \"tech:go\"
name = \"Go\"
keywords = [\"go\"]
requires_any = [\"golang\", \"go module\"]
"
            .to_string(),
        )];
        let taxonomy = Taxonomy::load(&files).unwrap();
        let event = |title: &str| {
            let mut value: NormalizedEvent = serde_json::from_value(serde_json::json!({
                "schema_version": "1", "source": "s", "source_item_id": "1",
                "title": title, "kind": "news"
            }))
            .unwrap();
            value.summary = None;
            value
        };
        assert!(taxonomy.tag(&event("Time to go: upgrade now")).is_empty());
        assert_eq!(
            taxonomy
                .tag(&event("Go module proxy flaw in golang tooling"))
                .len(),
            1
        );
    }

    #[test]
    fn keywords_match_whole_words_only() {
        assert!(contains_word("pnpm: path traversal", "pnpm"));
        assert!(contains_word("issue in npm cli logs", "npm cli"));
        assert!(!contains_word("npmcli logs", "npm cli"));
        assert!(!contains_word("xpnpm tool", "pnpm"));
        assert!(!contains_word("pnpm2 tool", "pnpm"));
    }
}
