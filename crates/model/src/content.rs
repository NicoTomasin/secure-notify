// SPDX-License-Identifier: MIT OR Apache-2.0

//! Content files: taxonomy entities (docs/architecture/taxonomy.md#entity-files), validated by
//! `content verify` and by CI (docs/architecture/taxonomy.md#contribution-rules).

use std::collections::{BTreeMap, BTreeSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::validate::check_purl;

/// One taxonomy entity, from `content/taxonomy/<kind>/<slug>.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Taxonomy entity")]
pub struct TaxonomyEntity {
    /// Stable ID: `tech:<slug>`, `product:<vendor>/<slug>`, `vendor:<slug>`, or `org:<slug>`.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Parent entity (a product's vendor, a technology's ecosystem).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// Other names, accepted by `/watch`.
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Words that tag free text with this entity.
    #[serde(default)]
    pub keywords: Vec<String>,
    /// Qualifiers: when set, a keyword tags text only if one of these also appears.
    #[serde(default)]
    pub requires_any: Vec<String>,
    /// Exact mappings to standard identifiers.
    #[serde(default)]
    pub mappings: Mappings,
}

/// Exact mappings of an entity.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Mappings {
    /// Package URLs without version, such as `pkg:npm/typescript`.
    #[serde(default)]
    pub purl: Vec<String>,
    /// CPE 2.3 prefixes, such as `cpe:2.3:o:fortinet:fortios`.
    #[serde(default)]
    pub cpe: Vec<String>,
    /// Domains of an organization.
    #[serde(default)]
    pub domains: Vec<String>,
    /// Status page URL of an organization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statuspage: Option<String>,
}

/// Normalizes a keyword or text for matching.
pub fn normalize_text(text: &str) -> String {
    text.to_lowercase()
}

/// Directory and ID prefix of each entity kind.
const KINDS: &[(&str, &str)] = &[
    ("technologies", "tech:"),
    ("products", "product:"),
    ("vendors", "vendor:"),
    ("organizations", "org:"),
];

fn is_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// The ID a file at `path` (relative to `content/taxonomy/`, with `/` separators) must have.
fn expected_id(path: &str) -> Option<String> {
    let stem = path.strip_suffix(".toml")?;
    let (directory, rest) = stem.split_once('/')?;
    let (_, prefix) = KINDS.iter().find(|(dir, _)| *dir == directory)?;
    let parts: Vec<&str> = rest.split('/').collect();
    let valid = match (*prefix, parts.as_slice()) {
        ("product:", [vendor, product]) => is_slug(vendor) && is_slug(product),
        ("product:", _) => false,
        (_, [slug]) => is_slug(slug),
        _ => false,
    };
    valid.then(|| format!("{prefix}{rest}"))
}

/// Parses and validates taxonomy files, given as paths relative to `content/taxonomy/` and their
/// text. Returns the entities, or one problem per line.
pub fn load_taxonomy(files: &[(String, String)]) -> Result<Vec<TaxonomyEntity>, Vec<String>> {
    let mut problems = Vec::new();
    let mut entities = Vec::new();
    for (path, text) in files {
        let shown = format!("content/taxonomy/{path}");
        let entity: TaxonomyEntity = match toml::from_str(text) {
            Ok(entity) => entity,
            Err(e) => {
                problems.push(format!("{shown}: {e}"));
                continue;
            }
        };
        match expected_id(path) {
            Some(id) if id == entity.id => {}
            Some(id) => problems.push(format!("{shown}: id must be `{id}` for this path")),
            None => problems.push(format!(
                "{shown}: not a valid entity path (technologies/<slug>.toml, \
                 products/<vendor>/<slug>.toml, vendors/<slug>.toml, organizations/<slug>.toml)"
            )),
        }
        if entity.name.trim().is_empty() {
            problems.push(format!("{shown}: name must not be empty"));
        }
        for purl in &entity.mappings.purl {
            if let Err(reason) = check_purl(purl) {
                problems.push(format!("{shown}: mappings.purl: {reason}"));
            }
        }
        for keyword in entity.keywords.iter().chain(&entity.requires_any) {
            if keyword.trim().is_empty() || *keyword != normalize_text(keyword) {
                problems.push(format!(
                    "{shown}: keyword `{keyword}` must be lowercase and non-empty"
                ));
            }
        }
        entities.push((shown, entity));
    }

    let mut ids = BTreeSet::new();
    for (shown, entity) in &entities {
        if !ids.insert(entity.id.as_str()) {
            problems.push(format!("{shown}: duplicate id `{}`", entity.id));
        }
    }
    for (shown, entity) in &entities {
        if let Some(parent) = &entity.parent
            && !ids.contains(parent.as_str())
        {
            problems.push(format!("{shown}: parent `{parent}` does not exist"));
        }
    }
    // A keyword shared by entities needs qualifiers on every one of them.
    let mut by_keyword: BTreeMap<&str, Vec<&TaxonomyEntity>> = BTreeMap::new();
    for (_, entity) in &entities {
        for keyword in &entity.keywords {
            by_keyword.entry(keyword.as_str()).or_default().push(entity);
        }
    }
    for (keyword, owners) in by_keyword {
        if owners.len() > 1 && owners.iter().any(|e| e.requires_any.is_empty()) {
            let names: Vec<&str> = owners.iter().map(|e| e.id.as_str()).collect();
            problems.push(format!(
                "keyword `{keyword}` is shared by {} without qualifiers (requires_any)",
                names.join(", ")
            ));
        }
    }
    if problems.is_empty() {
        Ok(entities.into_iter().map(|(_, e)| e).collect())
    } else {
        Err(problems)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, text: &str) -> (String, String) {
        (path.to_string(), text.to_string())
    }

    const NODE: &str = "id = \"tech:node\"\nname = \"Node.js\"\n";

    fn problems(files: &[(String, String)]) -> Vec<String> {
        load_taxonomy(files).err().unwrap_or_default()
    }

    #[test]
    fn contribution_rules_are_enforced() {
        // taxonomy.md#contribution-rules: unique IDs, valid mappings, existing parents, no
        // keyword collisions without qualifiers; IDs match their paths.
        let cases: [(&str, Vec<(String, String)>, &str); 7] = [
            (
                "duplicate id",
                vec![
                    file("technologies/node.toml", NODE),
                    file("technologies/node.toml", NODE),
                ],
                "duplicate id",
            ),
            (
                "missing parent",
                vec![file(
                    "technologies/node.toml",
                    "id = \"tech:node\"\nname = \"Node.js\"\nparent = \"tech:javascript\"\n",
                )],
                "parent `tech:javascript` does not exist",
            ),
            (
                "id not matching path",
                vec![file("technologies/nodejs.toml", NODE)],
                "id must be `tech:nodejs`",
            ),
            (
                "purl with version",
                vec![file(
                    "technologies/node.toml",
                    "id = \"tech:node\"\nname = \"Node.js\"\n[mappings]\npurl = [\"pkg:npm/node@20.0.0\"]\n",
                )],
                "must not contain a version",
            ),
            (
                "unknown field",
                vec![file(
                    "technologies/node.toml",
                    "id = \"tech:node\"\nname = \"Node.js\"\ndescription = \"x\"\n",
                )],
                "unknown field",
            ),
            (
                "keyword collision",
                vec![
                    file(
                        "technologies/a.toml",
                        "id = \"tech:a\"\nname = \"A\"\nkeywords = [\"shared\"]\n",
                    ),
                    file(
                        "technologies/b.toml",
                        "id = \"tech:b\"\nname = \"B\"\nkeywords = [\"shared\"]\nrequires_any = [\"b\"]\n",
                    ),
                ],
                "shared by tech:a, tech:b without qualifiers",
            ),
            (
                "uppercase keyword",
                vec![file(
                    "technologies/node.toml",
                    "id = \"tech:node\"\nname = \"Node.js\"\nkeywords = [\"Node.js\"]\n",
                )],
                "must be lowercase",
            ),
        ];
        for (case, files, expected) in cases {
            let found = problems(&files);
            assert!(
                found.iter().any(|p| p.contains(expected)),
                "{case}: {found:?}"
            );
        }
    }

    #[test]
    fn qualified_shared_keywords_and_products_under_vendors_are_valid() {
        let files = [
            file(
                "technologies/a.toml",
                "id = \"tech:a\"\nname = \"A\"\nkeywords = [\"shared\"]\nrequires_any = [\"a\"]\n",
            ),
            file(
                "technologies/b.toml",
                "id = \"tech:b\"\nname = \"B\"\nkeywords = [\"shared\"]\nrequires_any = [\"b\"]\n",
            ),
            file(
                "vendors/fortinet.toml",
                "id = \"vendor:fortinet\"\nname = \"Fortinet\"\n",
            ),
            file(
                "products/fortinet/fortios.toml",
                "id = \"product:fortinet/fortios\"\nname = \"FortiOS\"\nparent = \"vendor:fortinet\"\n",
            ),
        ];
        assert_eq!(problems(&files), Vec::<String>::new());
    }
}
