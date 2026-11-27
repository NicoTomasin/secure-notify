// SPDX-License-Identifier: AGPL-3.0-or-later

//! Tagging with the repository's taxonomy (docs/architecture/taxonomy.md#tagging) and watches
//! through the bot commands.

#![allow(clippy::unwrap_used)]

mod common;

use common::{Instance, repository_taxonomy};
use secure_notify_model::{
    Affected, Component, Entities, EventKind, Identifiers, NormalizedEvent, Severity,
};

/// An event with a real advisory summary as title and its real affected npm packages.
fn advisory(title: &str, packages: &[&str]) -> NormalizedEvent {
    NormalizedEvent {
        schema_version: "1".to_string(),
        source: "osv".to_string(),
        source_item_id: "test".to_string(),
        url: None,
        title: title.to_string(),
        summary: None,
        published_at: None,
        updated_at: None,
        kind: EventKind::Vulnerability,
        identifiers: Identifiers::default(),
        links: Vec::new(),
        affected: packages
            .iter()
            .map(|p| Affected {
                component: Component {
                    purl: Some(format!("pkg:npm/{}", p.replace('@', "%40"))),
                    cpe: None,
                },
                ranges: Vec::new(),
                fixed: Vec::new(),
            })
            .collect(),
        entities: Entities::default(),
        severity: Severity::default(),
        breach: None,
        incident: None,
    }
}

#[test]
fn real_advisories_are_tagged_as_expected() {
    // Summaries and affected packages from the GitHub Advisory Database, retrieved 2026-11-27
    // (https://github.com/advisories/<ID>); Node.js title from the Node.js vulnerability feed.
    let taxonomy = repository_taxonomy();
    let mut crossenv = advisory("crossenv is malware", &["crossenv"]);
    // OSV details of GHSA-c2m4-w5hm-vqjw (api.osv.dev, retrieved 2026-11-27): the word "npm"
    // alone must not tag the npm CLI.
    crossenv.summary = Some(
        "The `crossenv` package is a piece of malware that steals environment variables and sends \n         them to attacker controlled locations. All versions have been unpublished from the npm \n         registry."
            .to_string(),
    );
    let cases: [(&str, NormalizedEvent, &[&str]); 7] = [
        ("GHSA-c2m4-w5hm-vqjw", crossenv, &[]),
        (
            "GHSA-c59q-g84q-2gj5",
            advisory(
                "pnpm: Virtual store linker path traversal via unvalidated depPath name in lockfileToDepGraph",
                &["pnpm"],
            ),
            &["tech:pnpm"],
        ),
        (
            "GHSA-93f3-23rq-pjfp",
            advisory(
                "npm CLI exposing sensitive information through logs",
                &["npm"],
            ),
            &["tech:npm"],
        ),
        (
            "GHSA-v9mx-4pqq-h232",
            advisory(
                "Bun has an Application-level Prototype Pollution vulnerability in the runtime native API for Glo",
                &["bun"],
            ),
            &["tech:bun"],
        ),
        (
            // `javascript:` URLs are a common advisory topic, not the JavaScript technology.
            "GHSA-6hgp-fmhr-mrg4",
            advisory(
                "Highcharts Grid: `javascript:` URL in credits.href runs script on click",
                &["@highcharts/dashboards", "@highcharts/grid-pro"],
            ),
            &[],
        ),
        (
            // A library written in TypeScript is not a TypeScript vulnerability.
            "GHSA-f897-62hf-6gxq",
            advisory(
                "MCP TypeScript SDK: HTTP server transports and handlers did not limit request body size or JSON-RPC batch length",
                &["@modelcontextprotocol/sdk", "@modelcontextprotocol/node"],
            ),
            &[],
        ),
        (
            "Node.js feed",
            advisory("Wednesday, November 11, 2026 Security Releases", &[]),
            &[],
        ),
    ];
    for (case, event, expected) in cases {
        let tags: Vec<String> = taxonomy.tag(&event).into_iter().collect();
        assert_eq!(tags, expected, "{case}");
    }
}

#[test]
fn entities_set_by_the_source_are_not_repeated_as_core_tags() {
    // taxonomy.md: tags added by the core are recorded separately from tags set by the source.
    let taxonomy = repository_taxonomy();
    let mut event = advisory("pnpm: path traversal", &["pnpm"]);
    event.entities.technologies = vec!["tech:pnpm".to_string()];
    assert!(taxonomy.tag(&event).is_empty());
}

#[tokio::test]
async fn watching_by_name_alias_or_id() {
    let instance = Instance::with_admin().await;
    assert_eq!(
        instance.send("alice", "/watch node.js").await.unwrap(),
        "You now watch Node.js."
    );
    assert_eq!(
        instance.send("alice", "/watch tech:pnpm").await.unwrap(),
        "You now watch pnpm."
    );
    assert_eq!(
        instance.send("alice", "/watch TS").await.unwrap(),
        "You now watch TypeScript."
    );
    assert_eq!(
        instance.send("alice", "/watch Node").await.unwrap(),
        "You already watch Node.js."
    );
    let unknown = instance.send("alice", "/watch php").await.unwrap();
    assert!(
        unknown.starts_with("“php” is not in the catalog."),
        "{unknown}"
    );
    assert_eq!(
        instance.send("alice", "/watching").await.unwrap(),
        "You watch:\n• Node.js (tech:node)\n• pnpm (tech:pnpm)\n• TypeScript (tech:typescript)"
    );
    assert_eq!(
        instance.send("alice", "/unwatch pnpm").await.unwrap(),
        "You no longer watch pnpm."
    );
    assert_eq!(
        instance.send("alice", "/unwatch pnpm").await.unwrap(),
        "You do not watch pnpm."
    );
}

#[tokio::test]
async fn derived_watches_never_replace_manual_ones() {
    let instance = Instance::with_admin().await;
    let watches = &instance.conversation.core().watches;
    let alice = instance.conversation.subscribers().list().await.unwrap()[0]
        .subscriber
        .id;
    instance.send("alice", "/watch npm").await.unwrap();
    watches
        .derive(alice, &["tech:npm".to_string(), "tech:node".to_string()])
        .await
        .unwrap();
    assert_eq!(
        instance.send("alice", "/watching").await.unwrap(),
        "You watch:\n• Node.js (tech:node) · from your uploads\n• npm (tech:npm)"
    );
    // Watching a derived entity makes it manual.
    instance.send("alice", "/watch node.js").await.unwrap();
    assert_eq!(
        instance.send("alice", "/watching").await.unwrap(),
        "You watch:\n• Node.js (tech:node)\n• npm (tech:npm)"
    );
}
