// SPDX-License-Identifier: AGPL-3.0-or-later

//! Fixture tests; provenance in fixtures/README.md. Expected components were read by hand from
//! each fixture.

#![allow(clippy::unwrap_used)]

use secure_notify_core::inventory::{ParseError, Parsed, Parser};
use secure_notify_parser_npm::NpmParser;

fn parse(name: &str, fixture: &str) -> Parsed {
    let path = format!("{}/fixtures/{fixture}", env!("CARGO_MANIFEST_DIR"));
    let bytes = std::fs::read(path).unwrap();
    NpmParser.parse(name, &bytes).unwrap()
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
fn lockfile_v3_yields_exact_versions_including_nested_paths_but_not_the_project() {
    let parsed = parse("package-lock.json", "package-lock-v3.json");
    assert_eq!(
        pairs(&parsed),
        [
            "pkg:npm/%40octokit/auth-app 8.0.0",
            "pkg:npm/%40octokit/endpoint 10.1.4",
            "pkg:npm/before-after-hook 4.0.0",
            "pkg:npm/universal-user-agent 7.0.3",
        ]
    );
    // The root entry names the project (`@octokit/core`); it must not be emitted.
    assert!(!pairs(&parsed).iter().any(|p| p.contains("octokit/core")));
    assert_eq!(parsed.technologies, ["tech:npm", "tech:javascript"]);
}

#[test]
fn shrinkwrap_v3_is_read_like_a_lock_file() {
    let parsed = parse("npm-shrinkwrap.json", "npm-shrinkwrap-v3.json");
    assert_eq!(
        pairs(&parsed),
        [
            "pkg:npm/%40types/bun 1.3.10",
            "pkg:npm/%40types/node 25.5.0",
            "pkg:npm/bun-types 1.3.10",
            "pkg:npm/clipanion 4.0.0-rc.4",
            "pkg:npm/typanion 3.14.0",
            "pkg:npm/typescript 5.9.3",
            "pkg:npm/undici-types 7.18.2",
            "pkg:npm/yaml 2.8.2",
        ]
    );
}

#[test]
fn lockfile_v2_uses_the_packages_map() {
    let parsed = parse("npm-shrinkwrap.json", "npm-shrinkwrap-v2.json");
    assert_eq!(
        pairs(&parsed),
        [
            "pkg:npm/asynckit 0.4.0",
            "pkg:npm/axios 0.27.2",
            "pkg:npm/combined-stream 1.0.8",
            "pkg:npm/delayed-stream 1.0.0",
            "pkg:npm/follow-redirects 1.14.9",
            "pkg:npm/form-data 4.0.0",
            "pkg:npm/mime-db 1.52.0",
            "pkg:npm/mime-types 2.1.35",
        ]
    );
}

#[test]
fn lockfile_v1_reads_nested_dependencies() {
    let parsed = parse("package-lock.json", "package-lock-v1.json");
    assert_eq!(
        pairs(&parsed),
        [
            "pkg:npm/acorn 6.1.1",
            "pkg:npm/anymatch 2.0.0",
            "pkg:npm/bl 1.0.3",
            "pkg:npm/normalize-path 2.1.1",
            "pkg:npm/process-nextick-args 1.0.7",
            "pkg:npm/readable-stream 2.0.6",
            "pkg:npm/string_decoder 0.10.31",
        ]
    );
}

#[test]
fn package_json_yields_components_without_versions() {
    // inventory.md: manifests with ranges produce entries without a version.
    let parsed = parse("package.json", "package.json");
    assert_eq!(
        pairs(&parsed),
        [
            "pkg:npm/%40types/estree -",
            "pkg:npm/typescript -",
            "pkg:npm/uvu -",
        ]
    );
    assert_eq!(parsed.technologies, ["tech:javascript"]);
}

#[test]
fn non_registry_and_aliased_dependencies_in_package_json() {
    // Synthetic: one specifier of each kind found in manifests.
    let manifest = br#"{
        "name": "example-project",
        "dependencies": {
            "local": "file:../local",
            "repo": "github:example/repo",
            "shorthand": "example/repo",
            "ws": "workspace:*",
            "renamed": "npm:@scope/real@^2.0.0",
            "plain": "~1.0.0"
        },
        "engines": { "node": ">=20" },
        "packageManager": "pnpm@9.0.0"
    }"#;
    let parsed = NpmParser.parse("package.json", manifest).unwrap();
    assert_eq!(
        pairs(&parsed),
        ["pkg:npm/%40scope/real -", "pkg:npm/plain -"]
    );
    assert_eq!(
        parsed.technologies,
        ["tech:javascript", "tech:node", "tech:pnpm"]
    );
}

#[test]
fn malformed_and_deeply_nested_input_fails_cleanly() {
    // Synthetic adversarial inputs (security.md: hostile uploads).
    let deep = format!("{}{}", "[".repeat(10_000), "]".repeat(10_000));
    for bytes in [
        b"{not json".to_vec(),
        b"[]".to_vec(),
        deep.into_bytes(),
        b"{}".to_vec(),
    ] {
        let result = NpmParser.parse("package-lock.json", &bytes);
        assert!(
            matches!(result, Err(ParseError::Malformed(_))),
            "{result:?}"
        );
    }
}
