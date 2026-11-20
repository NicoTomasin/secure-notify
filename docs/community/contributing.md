# Contributing

How to contribute to secure-notify, from the smallest change (adding a feed with one TOML file) to
plugins in any language, platform integrations, translations, and core code. Every contribution
goes through a pull request and follows the docs in this folder; the docs are the source of truth
(see [`../../AGENTS.md`](../../AGENTS.md)). The lifecycle of a change is in
[../engineering/workflow.md](../engineering/workflow.md); each contribution path below has a
matching skill in the [skills index](../../.agents/skills/README.md).

## Contribution paths

> Status: `Proposed`

| You want to add | You write | Doc |
|---|---|---|
| A feed, simple API, status page, or simple page | One TOML file in `content/feeds/` (no code) | [../architecture/plugin-runtimes.md](../architecture/plugin-runtimes.md) |
| A technology, product, vendor, or organization | One TOML file in `content/taxonomy/` (no code) | [../architecture/taxonomy.md](../architecture/taxonomy.md) |
| A source, parser, enricher, or exposure provider with logic | A WASM plugin in any language with component tooling | [../architecture/plugins.md](../architecture/plugins.md) |
| A high-volume source or performance-critical parser | A native Rust plugin | [../architecture/plugin-runtimes.md](../architecture/plugin-runtimes.md) |
| A new platform | A native platform plugin | [../architecture/platforms.md](../architecture/platforms.md) |
| A translation | Fluent files in `locales/<lang>/` | [../architecture/delivery.md](../architecture/delivery.md) |
| A design change | A docs pull request first | [`../../AGENTS.md`](../../AGENTS.md) |
| Your own system, outside the repository | A canonical feed endpoint, in any language | [../architecture/feed-protocol.md](../architecture/feed-protocol.md) |

## Pull request requirements

> Status: `Proposed`

- Pull requests follow the branch and title rules in
  [../engineering/git-workflow.md](../engineering/git-workflow.md#pull-requests).
- Docs are updated in the same pull request when behavior, architecture, or a rule changes.
- The changed flow was verified locally and the evidence is in the pull request (see
  [../engineering/workflow.md](../engineering/workflow.md#local-verification)).
- Code plugins include fixtures and tests that follow [../engineering/testing.md](../engineering/testing.md);
  all content and manifests pass schema validation.
- New files carry the SPDX header and plugins declare their license (see
  [../product/licensing.md](../product/licensing.md)).
- CI passes (see [../engineering/conventions.md](../engineering/conventions.md)).
- Contributions are accepted under the license of the path they touch; there is no CLA.
- Contributions made with AI assistants or agents are welcome under the conditions in
  [../engineering/workflow.md](../engineering/workflow.md#ai-assisted-and-agent-contributions).

## Acceptance criteria for sources and plugins

> Status: `Proposed`

- The source is public and its terms allow automated access; robots rules and rate limits are
  respected; intervals are reasonable.
- The source respects the ethical boundaries in [../product/vision.md](../product/vision.md).
- `source_item_id` is stable; events validate; identifiers are extracted when present.
- No personal data is collected or emitted beyond what the event schema allows.
- Entity tagging is conservative (see [../architecture/taxonomy.md](../architecture/taxonomy.md)).
- Declared capabilities are minimal: only the hosts and secrets actually needed.

## Review

> Status: `Proposed`

- Pull requests are reviewed by a reviewer for the area or by a maintainer (see
  [governance.md](governance.md)).
- Content changes reach instances through content bundles soon after merge (see
  [../architecture/content-registry.md](../architecture/content-registry.md)), so they get the same
  scrutiny as code.

## External plugins

> Status: `Proposed`

Plugins maintained outside this repository build on permissively licensed SDKs and, under the
plugin interface exception, may use any license (see [../product/licensing.md](../product/licensing.md)).
Operators enable external plugins explicitly; they are not reviewed by the project.
