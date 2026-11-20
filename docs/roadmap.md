# Roadmap

From design to the full product. Phase 0 is the current work; Phase 1 is the MVP, focused on
JavaScript and TypeScript; the full scope is organized as capability tracks that can progress in
parallel once the MVP exists. Unless a section says otherwise, roadmap contents are `Proposed`;
reorder or change them only with maintainer confirmation.

## Phase 0: Design (current)

- Agent- and contributor-oriented documentation in `docs/` and `AGENTS.md`.
- Development harness: workflow, gitflow, testing, local development, migrations, and configuration
  rules in `docs/engineering/`; task-scoped agent skills and their index in `.agents/skills/`; the
  pull request template.
- Community files: `README.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`.
- Repository setup: issue and pull request templates (source request, platform request, bug,
  design change), private vulnerability reporting enabled, branch protection, GitHub Sponsors and
  the repository's funding configuration.

## Phase 1: MVP

### Scope

> Status: `Decided`

- **Coverage:** vulnerabilities affecting the JavaScript and TypeScript languages and their
  packages: packages from the npm registry installed with npm, pnpm, or Bun, and the languages'
  runtimes and tooling.
- **Platform:** Telegram (see the platform order in [architecture/platforms.md](architecture/platforms.md)).

### Contents

> Status: `Proposed`

| Area | In the MVP |
|---|---|
| Inventory | Parsers for `package.json`, `package-lock.json`, `npm-shrinkwrap.json`, `pnpm-lock.yaml`, `bun.lock`; manual entries; initial scan ([architecture/inventory.md](architecture/inventory.md)). |
| Taxonomy | JavaScript/TypeScript subset: `tech:javascript`, `tech:typescript`, `tech:node`, `tech:bun`, `tech:npm`, `tech:pnpm`; derived from uploads and watchable with `/watch`. |
| Sources | Local OSV mirror of the npm ecosystem (includes the GitHub Advisory Database and malicious package entries); Node.js and Bun security release feeds; CISA KEV for the `exploited` flag. |
| Plugin runtimes | Native plugins and the declarative `rss`/`atom` types; plugin contracts and manifests as specified, even though no external plugins exist yet. |
| Correlation | Identifiers and canonical URL; backfill; grace window; double check; retention ([architecture/deduplication.md](architecture/deduplication.md)). |
| Matching | `exact-version`, `component`, and `technology` relevance; npm version semantics; default rules and a minimum-severity setting ([architecture/matching-and-rules.md](architecture/matching-and-rules.md)). |
| Delivery | `instant` mode; localized messages with English as reference ([architecture/delivery.md](architecture/delivery.md)). |
| Subscribers | Person subscribers in private chats; pairing and invites ([architecture/subscribers.md](architecture/subscribers.md)). |
| Commands | `/start`, `/help`, `/about`, `/upload`, `/inventory`, `/watch`, `/unwatch`, `/watching`, `/rules`, `/language`, `/invite`, `/subscribers`, `/remove`, `/status`. |

Not in the MVP: WASM plugins, other ecosystems, products and organizations, incidents, exposure
monitoring, teams, other platforms, digests, web panel, federation, content registry updates.

### Foundations

The MVP MUST include from day one:

- the Cargo workspace skeleton, lints, tests, CI, cargo-deny, SPDX headers
  ([engineering/conventions.md](engineering/conventions.md));
- `cargo xtask ci` and the `dev` feature with the console platform, event ingestion, and the
  development clock, so every flow can be verified locally without third-party accounts
  ([engineering/local-development.md](engineering/local-development.md));
- the SQLite schema and migrations ([architecture/data-model.md](architecture/data-model.md));
- the canonical schema and plugin contracts, even if only built-in plugins exist
  ([architecture/plugins.md](architecture/plugins.md));
- the i18n layer, with English as reference ([architecture/delivery.md](architecture/delivery.md));
- the conversation layer, so Telegram is not hard-coded
  ([architecture/platforms.md](architecture/platforms.md));
- deduplication with the double check and the delivery ledger
  ([architecture/deduplication.md](architecture/deduplication.md)).

## Capability tracks (full scope)

Unordered beyond the MVP. Each item links to the doc that defines it.

| Track | Items |
|---|---|
| Core pipeline | Correlation with all signals, backfill, enrichers, matcher with inverted indexes, queue and dispatcher, update notices ([architecture/overview.md](architecture/overview.md)). |
| Vulnerability coverage | Local OSV mirror, NVD, CVE records, EUVD, distribution trackers, local version evaluation for every ecosystem ([architecture/matching-and-rules.md](architecture/matching-and-rules.md)). |
| Exploitation signals | CISA KEV, EPSS enricher, public exploit sources, `exploited` and `exploit_available` handling. |
| Incident coverage | Breach sources, attack and ransomware reporting, status pages, organization taxonomy. |
| Plugin platform | Declarative engine (all types), WASM host with capabilities and limits, WIT worlds, guest SDKs (Rust first, then other languages), `plugin test` and `plugin record` ([architecture/plugin-runtimes.md](architecture/plugin-runtimes.md)). |
| Content | Content bundles, signing, registry updates, additional registries ([architecture/content-registry.md](architecture/content-registry.md)). |
| Inventory | Parsers for every planned format, SBOM support, manual entries, initial scans ([architecture/inventory.md](architecture/inventory.md)). |
| Taxonomy | Technologies, products, vendors, organizations; tagging with qualifiers ([architecture/taxonomy.md](architecture/taxonomy.md)). |
| Exposure monitoring | Identity verification, encryption, exposure providers ([architecture/exposure-monitoring.md](architecture/exposure-monitoring.md)). |
| Subscribers | Teams, endpoint linking, invites, open registration, multiple admins ([architecture/subscribers.md](architecture/subscribers.md)). |
| Platforms | Telegram; then email and webhook; then Slack, Discord, and WhatsApp; later Matrix, Mattermost, ntfy, Gotify, Signal ([architecture/platforms.md](architecture/platforms.md)). |
| Delivery | Per-severity modes, routing, digests, quiet hours, inline actions, message splitting ([architecture/delivery.md](architecture/delivery.md)). |
| Web panel | HTTP server, login links, subscriber and admin pages, health and metrics ([architecture/web-panel.md](architecture/web-panel.md)). |
| Federation | Canonical feed publishing and consuming ([architecture/feed-protocol.md](architecture/feed-protocol.md)). |
| Operations | Backups, migrations with automatic backup, `/status`, source health, retention, release artifacts and signing ([operations/self-hosting.md](operations/self-hosting.md), [engineering/migrations.md](engineering/migrations.md), [engineering/releases.md](engineering/releases.md)). |
| Performance | Benchmarks, workload generator, budget checks in CI ([architecture/performance.md](architecture/performance.md)). |
| Community | Plugin authoring guides per language, contributor tooling, translations, governance as the team grows ([community/contributing.md](community/contributing.md)). |

## Later / maybe

- Optional LLM plugin (out of scope for the core; see [product/vision.md](product/vision.md)).
- WASM platform plugins, once the WASM runtime handles long-lived connections well.
- Periodic sync of manifests from a Git repository instead of manual uploads.
- Encrypted Matrix rooms and other platform extensions.
