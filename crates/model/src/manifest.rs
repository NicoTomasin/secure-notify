// SPDX-License-Identifier: MIT OR Apache-2.0

//! The plugin manifest, `plugin.toml` (docs/architecture/plugins.md#plugin-manifest).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Kinds of plugins (docs/architecture/plugins.md#plugin-kinds).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PluginKind {
    /// Fetches items on a schedule.
    Source,
    /// Turns uploaded files into components.
    Parser,
    /// Adds data to stories.
    Enricher,
    /// Checks watched identities.
    ExposureProvider,
    /// Delivers messages and receives commands.
    Platform,
}

impl PluginKind {
    /// Directory under `plugins/` holding plugins of this kind.
    pub fn directory(self) -> &'static str {
        match self {
            Self::Source => "sources",
            Self::Parser => "parsers",
            Self::Enricher => "enrichers",
            Self::ExposureProvider => "exposure",
            Self::Platform => "platforms",
        }
    }
}

/// How a plugin runs (docs/architecture/plugin-runtimes.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PluginRuntime {
    /// Compiled into the binary.
    Native,
    /// A WebAssembly component.
    Wasm,
}

/// Capabilities a plugin declares; the core enforces them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    /// The only hosts the plugin may contact.
    #[serde(default)]
    pub hosts: Vec<String>,
    /// Named secrets the operator must provide.
    #[serde(default)]
    pub secrets: Vec<String>,
    /// Whether the plugin keeps persistent state.
    #[serde(default)]
    pub state: bool,
}

/// `plugin.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Plugin manifest")]
pub struct PluginManifest {
    /// Plugin ID: lowercase ASCII letters, digits, and `-`; equal to its directory name.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Plugin kind.
    pub kind: PluginKind,
    /// Runtime.
    pub runtime: PluginRuntime,
    /// WIT world the plugin implements; required for WASM plugins.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api: Option<String>,
    /// Plugin version.
    pub version: String,
    /// SPDX license expression.
    pub license: String,
    /// Schedule, for sources and scheduled enrichers (`1h`, `30m`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval: Option<String>,
    /// Whether the plugin is enabled when the operator says nothing.
    #[serde(default = "enabled")]
    pub default_enabled: bool,
    /// Whether the source mirrors a whole database (exempt from retention).
    #[serde(default)]
    pub mirror: bool,
    /// Declared capabilities.
    #[serde(default)]
    pub capabilities: Capabilities,
}

fn enabled() -> bool {
    true
}

fn is_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Parses and validates a manifest found at `plugins/<directory>/<dir_name>/plugin.toml`.
pub fn load_manifest(
    directory: &str,
    dir_name: &str,
    text: &str,
) -> Result<PluginManifest, Vec<String>> {
    let manifest: PluginManifest = toml::from_str(text).map_err(|e| vec![e.to_string()])?;
    let mut problems = Vec::new();
    if !is_id(&manifest.id) {
        problems.push(format!(
            "id `{}` must be lowercase letters, digits, and `-`",
            manifest.id
        ));
    }
    if manifest.id != dir_name {
        problems.push(format!(
            "id `{}` must equal the directory name `{dir_name}`",
            manifest.id
        ));
    }
    if manifest.kind.directory() != directory {
        problems.push(format!(
            "a {:?} plugin belongs in plugins/{}/",
            manifest.kind,
            manifest.kind.directory()
        ));
    }
    if manifest.name.trim().is_empty() || manifest.license.trim().is_empty() {
        problems.push("name and license must not be empty".to_string());
    }
    if manifest.runtime == PluginRuntime::Wasm && manifest.api.is_none() {
        problems.push("WASM plugins must declare `api`".to_string());
    }
    let scheduled = matches!(manifest.kind, PluginKind::Source | PluginKind::Enricher);
    if manifest.interval.is_some() && !scheduled {
        problems.push("only sources and enrichers have an `interval`".to_string());
    }
    if manifest.mirror && manifest.kind != PluginKind::Source {
        problems.push("only sources can set `mirror`".to_string());
    }
    if problems.is_empty() {
        Ok(manifest)
    } else {
        Err(problems)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PARSER: &str = "id = \"npm\"\nname = \"npm\"\nkind = \"parser\"\nruntime = \"native\"\nversion = \"0.1.0\"\nlicense = \"AGPL-3.0-or-later\"\n";

    #[test]
    fn manifest_rules_are_enforced() {
        assert!(load_manifest("parsers", "npm", PARSER).is_ok());
        let cases = [
            ("parsers", "pnpm", PARSER, "must equal the directory name"),
            ("sources", "npm", PARSER, "belongs in plugins/parsers/"),
            (
                "parsers",
                "npm",
                &format!("{PARSER}interval = \"1h\"\n"),
                "only sources and enrichers",
            ),
            (
                "parsers",
                "npm",
                &format!("{PARSER}mirror = true\n"),
                "only sources can set",
            ),
            (
                "parsers",
                "npm",
                &PARSER.replace("native", "wasm"),
                "must declare `api`",
            ),
            (
                "parsers",
                "npm",
                &format!("{PARSER}color = \"red\"\n"),
                "unknown field",
            ),
        ];
        for (directory, dir_name, text, expected) in cases {
            let problems = load_manifest(directory, dir_name, text).unwrap_err();
            assert!(
                problems.iter().any(|p| p.contains(expected)),
                "{expected}: {problems:?}"
            );
        }
    }
}
