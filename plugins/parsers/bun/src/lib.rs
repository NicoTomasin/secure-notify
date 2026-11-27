// SPDX-License-Identifier: AGPL-3.0-or-later

//! Parser for Bun's text lockfile, `bun.lock`. The binary lockfile `bun.lockb` is not parsed;
//! the bot explains how to produce `bun.lock` instead (docs/architecture/inventory.md).

use secure_notify_core::inventory::{ParseError, Parsed, ParsedComponent, Parser};
use secure_notify_model::npm_purl;
use serde_json::Value;

/// The Bun parser plugin.
#[derive(Debug, Default, Clone, Copy)]
pub struct BunParser;

/// `bun.lock` is JSON with trailing commas; removes commas that precede `}` or `]` outside
/// strings.
fn strip_trailing_commas(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let (mut in_string, mut escaped) = (false, false);
    for (i, &c) in chars.iter().enumerate() {
        if in_string {
            out.push(c);
            match (escaped, c) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => in_string = false,
                _ => {}
            }
            continue;
        }
        if c == '"' {
            in_string = true;
        } else if c == ',' {
            let next = chars[i + 1..].iter().find(|n| !n.is_whitespace());
            if matches!(next, Some('}' | ']')) {
                continue;
            }
        }
        out.push(c);
    }
    out
}

impl Parser for BunParser {
    fn id(&self) -> &'static str {
        "bun"
    }

    fn accepts(&self, filename: &str) -> bool {
        matches!(filename, "bun.lock" | "bun.lockb")
    }

    fn parse(&self, filename: &str, bytes: &[u8]) -> Result<Parsed, ParseError> {
        if filename == "bun.lockb" {
            return Err(ParseError::BunBinaryLockfile);
        }
        let malformed = || ParseError::Malformed("not a Bun lock file".to_string());
        let text = std::str::from_utf8(bytes).map_err(|_| malformed())?;
        let value: Value =
            serde_json::from_str(&strip_trailing_commas(text)).map_err(|_| malformed())?;
        let packages = value
            .get("packages")
            .and_then(Value::as_object)
            .ok_or_else(malformed)?;
        let mut components = Vec::new();
        for entry in packages.values() {
            // Each entry is an array whose first element is `name@version`.
            let Some(spec) = entry.get(0).and_then(Value::as_str) else {
                continue;
            };
            let Some(at) = spec.get(1..).and_then(|s| s.find('@')).map(|i| i + 1) else {
                continue;
            };
            let (name, version) = (&spec[..at], &spec[at + 1..]);
            // Workspace, file, and Git packages have no registry version.
            if version.starts_with(|c: char| c.is_ascii_digit()) {
                components.push(ParsedComponent {
                    purl: npm_purl(name),
                    version: Some(version.to_string()),
                });
            }
        }
        Ok(Parsed {
            components,
            technologies: vec!["tech:bun".to_string(), "tech:javascript".to_string()],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trailing_commas_are_removed_outside_strings_only() {
        assert_eq!(
            strip_trailing_commas("{\"a\": [1, 2,], \"b\": \",]\",}"),
            "{\"a\": [1, 2], \"b\": \",]\"}"
        );
    }
}
