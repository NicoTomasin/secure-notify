// SPDX-License-Identifier: AGPL-3.0-or-later

//! Parser for `pnpm-lock.yaml` (lockfile versions 5, 6, and 9): exact versions of every package,
//! read from the keys of the `packages` map.

use std::collections::BTreeMap;

use secure_notify_core::inventory::{ParseError, Parsed, ParsedComponent, Parser};
use secure_notify_model::npm_purl;
use serde::Deserialize;
use serde::de::IgnoredAny;

/// The pnpm parser plugin.
#[derive(Debug, Default, Clone, Copy)]
pub struct PnpmParser;

#[derive(Deserialize)]
#[serde(untagged)]
enum LockfileVersion {
    Number(f64),
    Text(String),
}

#[derive(Deserialize)]
struct Lockfile {
    #[serde(rename = "lockfileVersion")]
    lockfile_version: LockfileVersion,
    // Only the keys are read; values are skipped without being stored.
    #[allow(clippy::zero_sized_map_values)]
    #[serde(default)]
    packages: BTreeMap<String, IgnoredAny>,
}

/// Name and version from a `packages` key, or `None` for packages that are not from the
/// registry (links, tarballs, Git, runtimes).
///
/// - Version 5: `/name/1.0.0` or `/@scope/name/1.0.0_peer@2.0.0`.
/// - Version 6: `/name@1.0.0` or `/@scope/name@1.0.0(peer@2.0.0)`.
/// - Version 9: `name@1.0.0` or `@scope/name@1.0.0`.
fn name_and_version(key: &str, slash_separated: bool) -> Option<(&str, &str)> {
    let key = key.split('(').next().unwrap_or(key);
    let (name, version) = if slash_separated {
        let key = key.strip_prefix('/')?;
        let (name, version) = key.rsplit_once('/')?;
        (name, version.split('_').next().unwrap_or(version))
    } else {
        let key = key.strip_prefix('/').unwrap_or(key);
        let at = key.get(1..)?.find('@')? + 1;
        (&key[..at], &key[at + 1..])
    };
    let registry = !name.is_empty() && version.starts_with(|c: char| c.is_ascii_digit());
    registry.then_some((name, version))
}

impl Parser for PnpmParser {
    fn id(&self) -> &'static str {
        "pnpm"
    }

    fn accepts(&self, filename: &str) -> bool {
        filename == "pnpm-lock.yaml"
    }

    fn parse(&self, _filename: &str, bytes: &[u8]) -> Result<Parsed, ParseError> {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| ParseError::Malformed("not UTF-8".to_string()))?;
        // serde-saphyr enforces budgets on aliases and nesting, which defeats YAML bombs.
        let lockfile: Lockfile = serde_saphyr::from_str(text)
            .map_err(|_| ParseError::Malformed("not a pnpm lock file".to_string()))?;
        let major = match &lockfile.lockfile_version {
            LockfileVersion::Number(n) => *n,
            LockfileVersion::Text(t) => t.parse().unwrap_or(0.0),
        };
        let slash_separated = major < 6.0;
        let components = lockfile
            .packages
            .keys()
            .filter_map(|key| name_and_version(key, slash_separated))
            .map(|(name, version)| ParsedComponent {
                purl: npm_purl(name),
                version: Some(version.to_string()),
            })
            .collect();
        Ok(Parsed {
            components,
            technologies: vec!["tech:pnpm".to_string(), "tech:javascript".to_string()],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_suffixes_are_removed_in_every_key_format() {
        // Hand-worked keys in the formats pnpm writes for packages resolved with peers.
        let cases = [
            ("/@babel/plugin-syntax-jsx/7.22.5_@babel+core@7.22.10", true),
            (
                "/@babel/plugin-syntax-jsx@7.22.5(@babel/core@7.22.10)",
                false,
            ),
            (
                "@babel/plugin-syntax-jsx@7.22.5(@babel/core@7.22.10)",
                false,
            ),
        ];
        for (key, slash_separated) in cases {
            assert_eq!(
                name_and_version(key, slash_separated),
                Some(("@babel/plugin-syntax-jsx", "7.22.5")),
                "{key}"
            );
        }
    }
}
