// SPDX-License-Identifier: AGPL-3.0-or-later

//! Fixture tests; provenance in fixtures/README.md. Expected components were read by hand from
//! the fixture.

#![allow(clippy::unwrap_used)]

use secure_notify_core::inventory::{ParseError, Parser};
use secure_notify_parser_bun::BunParser;

#[test]
fn bun_lock_yields_every_package_version_including_nested_duplicates() {
    let path = format!("{}/fixtures/bun.lock", env!("CARGO_MANIFEST_DIR"));
    let parsed = BunParser
        .parse("bun.lock", &std::fs::read(path).unwrap())
        .unwrap();
    let mut pairs: Vec<String> = parsed
        .components
        .iter()
        .map(|c| format!("{} {}", c.purl, c.version.as_deref().unwrap_or("-")))
        .collect();
    pairs.sort();
    assert_eq!(
        pairs,
        [
            "pkg:npm/%40clack/core 0.3.5",
            "pkg:npm/%40clack/prompts 0.7.0",
            "pkg:npm/%40types/bun 1.3.5",
            "pkg:npm/%40types/node 22.19.3",
            "pkg:npm/%40types/node 25.0.3",
            "pkg:npm/argparse 1.0.10",
            "pkg:npm/bun-types 1.3.5",
            "pkg:npm/chalk 5.6.2",
            "pkg:npm/esprima 4.0.1",
            "pkg:npm/extend-shallow 2.0.1",
            "pkg:npm/gray-matter 4.0.3",
            "pkg:npm/is-extendable 0.1.1",
            "pkg:npm/is-unicode-supported 2.1.0",
            "pkg:npm/js-yaml 3.14.2",
            "pkg:npm/kind-of 6.0.3",
            "pkg:npm/picocolors 1.1.1",
            "pkg:npm/section-matter 1.0.0",
            "pkg:npm/sisteransi 1.0.5",
            "pkg:npm/sprintf-js 1.0.3",
            "pkg:npm/strip-bom-string 1.0.0",
            "pkg:npm/typescript 5.9.3",
            "pkg:npm/undici-types 6.21.0",
            "pkg:npm/undici-types 7.16.0",
        ]
    );
    // The workspace name (`dotagents`) is the project; it must not be emitted.
    assert!(!pairs.iter().any(|p| p.contains("dotagents")));
    assert_eq!(parsed.technologies, ["tech:bun", "tech:javascript"]);
}

#[test]
fn binary_lockfile_is_refused_with_its_own_error() {
    // Synthetic bytes: the file name alone decides; the content is never read.
    assert_eq!(
        BunParser.parse("bun.lockb", b"#!/usr/bin/env bun\x00\x01"),
        Err(ParseError::BunBinaryLockfile)
    );
}

#[test]
fn workspace_and_git_packages_are_skipped() {
    // Synthetic entries of the kinds Bun writes for local and Git packages.
    let lock = br#"{
      "lockfileVersion": 1,
      "packages": {
        "app": ["app@workspace:packages/app"],
        "lib": ["lib@github:example/lib#0123abc", {}, "example-lib-0123abc"],
        "ok": ["ok@1.0.0", "", {}, "sha512-x"],
      },
    }"#;
    let parsed = BunParser.parse("bun.lock", lock).unwrap();
    assert_eq!(parsed.components.len(), 1);
    assert_eq!(parsed.components[0].purl, "pkg:npm/ok");
}
