// SPDX-License-Identifier: AGPL-3.0-or-later

//! Parser for npm files: `package.json` (components without version), `package-lock.json` and
//! `npm-shrinkwrap.json` (exact versions, transitive dependencies included; lockfile versions 1,
//! 2, and 3).
//!
//! Only component names and versions are read. The root package (the project itself) is never
//! emitted, so no project name leaves the file (docs/architecture/inventory.md#privacy).

use secure_notify_core::inventory::{ParseError, Parsed, ParsedComponent, Parser};
use secure_notify_model::npm_purl;
use serde_json::{Map, Value};

/// The npm parser plugin.
#[derive(Debug, Default, Clone, Copy)]
pub struct NpmParser;

/// Dependency sections of `package.json`.
const MANIFEST_SECTIONS: &[&str] = &[
    "dependencies",
    "devDependencies",
    "optionalDependencies",
    "peerDependencies",
];

/// A version string that starts like a semantic version; other values (URLs, `file:` paths,
/// tags) are not exact registry versions.
fn exact_version(text: &str) -> Option<String> {
    text.starts_with(|c: char| c.is_ascii_digit())
        .then(|| text.to_string())
}

/// Splits an alias `npm:<name>@<version>` into the real name and version.
fn alias(text: &str) -> Option<(&str, &str)> {
    let target = text.strip_prefix("npm:")?;
    let at = target[1..].find('@')? + 1;
    Some((&target[..at], &target[at + 1..]))
}

/// Whether a `package.json` dependency specifier refers to the npm registry.
fn registry_specifier(spec: &str) -> bool {
    !(spec.starts_with("file:")
        || spec.starts_with("link:")
        || spec.starts_with("workspace:")
        || spec.starts_with("portal:")
        || spec.starts_with("git")
        || spec.starts_with("http:")
        || spec.starts_with("https:")
        || spec.starts_with("github:")
        || (spec.contains('/') && !spec.starts_with("npm:")))
}

fn malformed(message: &str) -> ParseError {
    ParseError::Malformed(message.to_string())
}

fn parse_manifest(root: &Map<String, Value>) -> Parsed {
    let mut parsed = Parsed {
        technologies: vec!["tech:javascript".to_string()],
        ..Parsed::default()
    };
    for section in MANIFEST_SECTIONS {
        let Some(dependencies) = root.get(*section).and_then(Value::as_object) else {
            continue;
        };
        for (name, spec) in dependencies {
            let spec = spec.as_str().unwrap_or_default();
            if !registry_specifier(spec) {
                continue;
            }
            let name = alias(spec).map_or(name.as_str(), |(real, _)| real);
            parsed.components.push(ParsedComponent {
                purl: npm_purl(name),
                version: None,
            });
        }
    }
    if root
        .get("engines")
        .and_then(Value::as_object)
        .is_some_and(|e| e.contains_key("node"))
    {
        parsed.technologies.push("tech:node".to_string());
    }
    if let Some(manager) = root.get("packageManager").and_then(Value::as_str) {
        let name = manager.split('@').next().unwrap_or_default();
        if matches!(name, "npm" | "pnpm" | "bun") {
            parsed.technologies.push(format!("tech:{name}"));
        }
    }
    parsed
}

/// Lockfile versions 2 and 3: the flat `packages` map keyed by `node_modules/...` paths.
fn parse_packages(packages: &Map<String, Value>, parsed: &mut Parsed) {
    for (path, entry) in packages {
        // "" is the project itself; paths without `node_modules/` are workspace folders.
        let Some(index) = path.rfind("node_modules/") else {
            continue;
        };
        let Some(entry) = entry.as_object() else {
            continue;
        };
        if entry.get("link").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        let name = entry
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or(&path[index + "node_modules/".len()..]);
        if let Some(version) = entry.get("version").and_then(Value::as_str) {
            parsed.components.push(ParsedComponent {
                purl: npm_purl(name),
                version: exact_version(version),
            });
        }
    }
}

/// Lockfile version 1: nested `dependencies` maps.
fn parse_dependencies(dependencies: &Map<String, Value>, parsed: &mut Parsed) {
    let mut pending = vec![dependencies];
    while let Some(map) = pending.pop() {
        for (name, entry) in map {
            let Some(entry) = entry.as_object() else {
                continue;
            };
            if let Some(version) = entry.get("version").and_then(Value::as_str) {
                let (name, version) = alias(version).unwrap_or((name.as_str(), version));
                parsed.components.push(ParsedComponent {
                    purl: npm_purl(name),
                    version: exact_version(version),
                });
            }
            if let Some(nested) = entry.get("dependencies").and_then(Value::as_object) {
                pending.push(nested);
            }
        }
    }
}

impl Parser for NpmParser {
    fn id(&self) -> &'static str {
        "npm"
    }

    fn accepts(&self, filename: &str) -> bool {
        matches!(
            filename,
            "package.json" | "package-lock.json" | "npm-shrinkwrap.json"
        )
    }

    fn parse(&self, filename: &str, bytes: &[u8]) -> Result<Parsed, ParseError> {
        // serde_json limits nesting depth, so deeply nested input fails instead of overflowing.
        let value: Value =
            serde_json::from_slice(bytes).map_err(|_| malformed("not valid JSON"))?;
        let root = value
            .as_object()
            .ok_or_else(|| malformed("not a JSON object"))?;
        if filename == "package.json" {
            return Ok(parse_manifest(root));
        }
        let mut parsed = Parsed {
            technologies: vec!["tech:npm".to_string(), "tech:javascript".to_string()],
            ..Parsed::default()
        };
        match (
            root.get("packages").and_then(Value::as_object),
            root.get("dependencies").and_then(Value::as_object),
        ) {
            // Version 2 carries both maps; `packages` is the complete one.
            (Some(packages), _) => parse_packages(packages, &mut parsed),
            (None, Some(dependencies)) => parse_dependencies(dependencies, &mut parsed),
            (None, None) => return Err(malformed("no packages or dependencies")),
        }
        Ok(parsed)
    }
}
