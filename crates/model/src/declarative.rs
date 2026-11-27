// SPDX-License-Identifier: MIT OR Apache-2.0

//! Declarative source definitions, `content/feeds/<id>.toml`
//! (docs/architecture/plugin-runtimes.md#declarative-runtime).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::event::{EventKind, SeverityLevel};

/// Declarative source types. Only the feed types are implemented so far.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DeclarativeType {
    /// An RSS feed.
    Rss,
    /// An Atom feed.
    Atom,
}

/// Values applied to every event of the source.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Defaults {
    /// Event kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<EventKind>,
    /// Taxonomy entity IDs.
    #[serde(default)]
    pub entities: Vec<String>,
    /// Severity level when the item states none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<SeverityLevel>,
}

/// A declarative source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Declarative source")]
pub struct DeclarativeSource {
    /// Source ID: lowercase letters, digits, and `-`; equal to the file name; never changes.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Source type.
    #[serde(rename = "type")]
    pub kind: DeclarativeType,
    /// Feed URL (`https`).
    pub url: String,
    /// Schedule (`30m`, `1h`).
    pub interval: String,
    /// Whether the source is enabled when the operator says nothing.
    #[serde(default)]
    pub default_enabled: bool,
    /// Values applied to every event.
    #[serde(default)]
    pub defaults: Defaults,
}

/// Parses and validates a definition from `content/feeds/<file_stem>.toml`.
pub fn load_declarative_source(
    file_stem: &str,
    text: &str,
) -> Result<DeclarativeSource, Vec<String>> {
    let source: DeclarativeSource = toml::from_str(text).map_err(|e| vec![e.to_string()])?;
    let mut problems = Vec::new();
    let id_ok = !source.id.is_empty()
        && source
            .id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !id_ok || source.id != file_stem {
        problems.push(format!(
            "id `{}` must be lowercase letters, digits, and `-`, equal to the file name",
            source.id
        ));
    }
    if !source.url.starts_with("https://") {
        problems.push("url must be an https URL".to_string());
    }
    if source.name.trim().is_empty() {
        problems.push("name must not be empty".to_string());
    }
    let interval_ok = source
        .interval
        .strip_suffix(['s', 'm', 'h', 'd'])
        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
    if !interval_ok {
        problems.push(format!(
            "interval `{}` is not a duration such as 30m",
            source.interval
        ));
    }
    if problems.is_empty() {
        Ok(source)
    } else {
        Err(problems)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = "id = \"nodejs-security\"\nname = \"Node.js\"\ntype = \"rss\"\nurl = \"https://nodejs.org/en/feed/vulnerability.xml\"\ninterval = \"30m\"\n";

    #[test]
    fn definition_rules_are_enforced() {
        assert!(load_declarative_source("nodejs-security", VALID).is_ok());
        let cases = [
            ("other-name", VALID.to_string(), "equal to the file name"),
            (
                "nodejs-security",
                VALID.replace("https://", "http://"),
                "https URL",
            ),
            (
                "nodejs-security",
                VALID.replace("30m", "often"),
                "not a duration",
            ),
            (
                "nodejs-security",
                VALID.replace("rss", "html"),
                "unknown variant",
            ),
            (
                "nodejs-security",
                format!("{VALID}secret = \"X\"\n"),
                "unknown field",
            ),
        ];
        for (stem, text, expected) in cases {
            let problems = load_declarative_source(stem, &text).unwrap_err();
            assert!(
                problems.iter().any(|p| p.contains(expected)),
                "{expected}: {problems:?}"
            );
        }
    }
}
