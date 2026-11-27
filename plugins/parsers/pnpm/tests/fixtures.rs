// SPDX-License-Identifier: AGPL-3.0-or-later

//! Fixture tests; provenance in fixtures/README.md. Expected components were read by hand from
//! each fixture.

#![allow(clippy::unwrap_used)]

use std::fmt::Write as _;

use secure_notify_core::inventory::{ParseError, Parsed, Parser};
use secure_notify_parser_pnpm::PnpmParser;

fn parse(fixture: &str) -> Parsed {
    let path = format!("{}/fixtures/{fixture}", env!("CARGO_MANIFEST_DIR"));
    PnpmParser
        .parse("pnpm-lock.yaml", &std::fs::read(path).unwrap())
        .unwrap()
}

fn pairs(parsed: &Parsed) -> Vec<String> {
    let mut list: Vec<String> = parsed
        .components
        .iter()
        .map(|c| format!("{} {}", c.purl, c.version.as_deref().unwrap_or("-")))
        .collect();
    list.sort();
    list
}

#[test]
fn lockfile_v5_uses_slash_separated_keys() {
    assert_eq!(
        pairs(&parse("pnpm-lock-v5.yaml")),
        [
            "pkg:npm/%40types/estree 1.0.0",
            "pkg:npm/dequal 2.0.3",
            "pkg:npm/diff 5.1.0",
            "pkg:npm/kleur 4.1.5",
            "pkg:npm/mri 1.2.0",
            "pkg:npm/sade 1.8.1",
            "pkg:npm/typescript 4.9.4",
            "pkg:npm/uvu 0.5.6",
        ]
    );
}

#[test]
fn lockfile_v6_strips_peer_suffixes() {
    assert_eq!(
        pairs(&parse("pnpm-lock-v6.yaml")),
        [
            "pkg:npm/%40aashutoshrathi/word-wrap 1.2.6",
            "pkg:npm/%40antfu/eslint-config 2.1.1",
            "pkg:npm/%40babel/helper-module-transforms 7.22.9",
        ]
    );
}

#[test]
fn lockfile_v9_skips_non_registry_packages() {
    // `node@runtime:24.15.0` is a Node.js runtime pinned by pnpm, not the npm package `node`.
    let parsed = parse("pnpm-lock-v9.yaml");
    assert_eq!(pairs(&parsed), ["pkg:npm/zod 3.24.2"]);
    assert_eq!(parsed.technologies, ["tech:pnpm", "tech:javascript"]);
}

#[test]
fn yaml_bombs_and_garbage_fail_cleanly() {
    // Synthetic adversarial input: exponential alias expansion ("billion laughs").
    let mut bomb = String::from(
        "lockfileVersion: '9.0'\na: &a [\"x\",\"x\",\"x\",\"x\",\"x\",\"x\",\"x\",\"x\",\"x\"]\n",
    );
    let mut previous = 'a';
    for name in 'b'..='j' {
        let refs = vec![format!("*{previous}"); 9].join(",");
        let _ = writeln!(bomb, "{name}: &{name} [{refs}]");
        previous = name;
    }
    bomb.push_str("packages:\n  boom: *j\n");
    for input in [
        bomb.as_bytes(),
        b"{{{{",
        b"packages: [",
        &[0xff, 0xfe, 0x00][..],
    ] {
        let result = PnpmParser.parse("pnpm-lock.yaml", input);
        assert!(
            matches!(result, Err(ParseError::Malformed(_))),
            "{result:?}"
        );
    }
}
