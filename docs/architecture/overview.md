# Architecture overview

secure-notify is a single Rust binary organized as a pipeline: sources collect raw items, items
are normalized into events, events are correlated into stories, stories are enriched, matched
against each subscriber's watches, inventory, identities, and rules, and matching stories are
queued and delivered through platform plugins according to each subscriber's delivery
preferences. Everything is persisted in one SQLite database.

## Pipeline

> Status: `Proposed`

```
Sources ──▶ Normalizer ──▶ Correlator ──▶ Enrichers ──▶ Matcher ──▶ Queue ──▶ Dispatcher ──▶ Platforms
(plugins)   + validator    event→story    (plugins)        ▲        due_at    double check    (plugins)
                                                           │                                      │
                           watches, inventory, identities, rules, mutes                           │
                                                           ▲                                      ▼
            Parsers (uploads), Exposure providers ◀── commands, uploads, web panel ◀── interactive endpoints
```

| Stage | Responsibility | Doc |
|---|---|---|
| Sources | Fetch raw items on a schedule, keep per-source cursor state. | [plugins.md](plugins.md), [plugin-runtimes.md](plugin-runtimes.md) |
| Normalizer | Convert raw items to normalized events, validate against the schema, reject invalid ones without crashing. | [data-model.md](data-model.md) |
| Correlator | Attach each event to an existing story or create one; merge stories when identifiers reveal they are the same. | [deduplication.md](deduplication.md) |
| Enrichers | Add data to changed stories (EPSS, exploit availability, entity tagging). | [plugins.md](plugins.md) |
| Matcher | For each changed story, compute relevance per subscriber through inverted indexes and evaluate mutes and rules. | [matching-and-rules.md](matching-and-rules.md) |
| Queue | Hold pending notifications with a `due_at` computed from the subscriber's delivery preferences. | [delivery.md](delivery.md) |
| Dispatcher | Every minute, take due items, run the double check, render localized messages or digests, route to endpoints, record in the ledger. | [delivery.md](delivery.md), [deduplication.md](deduplication.md) |
| Platforms | Deliver rendered messages and receive commands, uploads, and button actions. | [platforms.md](platforms.md) |
| Parsers | Turn uploaded files, in memory, into anonymous components and technologies. | [inventory.md](inventory.md) |
| Exposure providers | Check verified watched identities against breach data. | [exposure-monitoring.md](exposure-monitoring.md) |

## Process model

> Status: `Proposed`

- One process, one async runtime (tokio). Long-lived tasks: scheduler, source runners, correlator,
  enrichers, matcher, dispatcher, one connector per enabled platform, content updater, and the
  optional HTTP server (see [web-panel.md](web-panel.md)).
- Stages communicate through bounded in-process channels backed by persisted state, so a restart
  never loses work: the database, not memory, is the source of truth for pending work.
- SQLite runs in WAL mode with a single writer task and a pool of read connections. CPU-heavy or
  blocking work (large uploads, data dumps, WASM calls) runs off the async executor.
- Admin CLI subcommands of the same binary (`run`, `migrate`, `backup`, `plugin test`,
  `plugin record`, `content verify`) cover operations and contributor tooling.

## Multi-tenancy model

> Status: `Decided` (multi-subscriber instance, persons and teams with linked endpoints) / `Proposed` (shared ingestion)

- One instance serves multiple subscribers: persons and teams, each with endpoints on one or more
  platforms. See [subscribers.md](subscribers.md).
- Ingestion, events, and stories are **shared** instance-wide: each source is fetched once,
  regardless of the number of subscribers.
- Watches, inventory, watched identities, rules, mutes, delivery preferences, language, queue
  entries, and the delivery ledger are **per subscriber**.

## Tech stack

> Status: `Decided`

| Concern | Choice |
|---|---|
| Language | Rust (stable, edition 2024) |
| Async runtime | tokio |
| Database | SQLite only, via rusqlite (bundled SQLite), WAL mode; storage behind a repository layer so the engine is not hard-wired into domain code |
| Migrations | embedded, forward-only, applied at startup (see [../engineering/migrations.md](../engineering/migrations.md)) |
| HTTP client | reqwest with rustls, wrapped by the core's rate-limited client |
| Feeds | feed-rs (RSS, Atom, JSON Feed) |
| HTML | scraper (CSS selectors, static HTML only; no headless browser) |
| JSON mapping | serde_json + serde_json_path (JSONPath for declarative JSON APIs) |
| Configuration and content | TOML via serde |
| WASM plugins | wasmtime with the component model, WIT interfaces |
| Telegram | teloxide |
| Web panel and HTTP surfaces | axum, server-rendered templates compiled into the binary, no JavaScript build toolchain |
| i18n | Fluent (`.ftl` files) |
| Logging | tracing with JSON output |
| Errors | thiserror in libraries |
| Tests | cargo test / nextest, insta (snapshots), proptest, wiremock |
| Benchmarks | criterion |
| Supply chain | cargo-deny |

## Repository layout

> Status: `Decided`

```
Cargo.toml                    # workspace
crates/
  secure-notify/              # the binary: CLI and wiring
  core/                       # pipeline, scheduler, correlation, matching, delivery
  model/                      # canonical types; JSON Schema generation (MIT OR Apache-2.0)
  storage/                    # SQLite persistence; migrations/ holds the embedded migrations
  plugin-host/                # WASM component host and capability enforcement
  declarative/                # declarative source engine
  conversation/               # commands, permissions, message model, i18n
  versions/                   # per-ecosystem version and range evaluation
  web/                        # optional HTTP server: panel, canonical feed, health, metrics
sdk/                          # MIT OR Apache-2.0
  wit/                        # WIT packages: the polyglot plugin contract
  rust/                       # Rust guest SDK (more languages added over time)
plugins/
  sources/<id>/               # native or WASM sources: plugin.toml + src + fixtures + tests
  parsers/<id>/
  enrichers/<id>/
  exposure/<id>/
  platforms/<id>/             # telegram, discord, ...
content/
  feeds/<id>.toml             # declarative sources
  taxonomy/<kind>/<id>.toml   # technologies, products, vendors, organizations
  rules/                      # keyword rules for tagging and severity
locales/<lang>/*.ftl          # bot translations (en is the reference)
schemas/                      # generated JSON Schemas (MIT OR Apache-2.0)
xtask/                        # development automation (cargo xtask), see engineering/local-development.md
docs/                         # rules: the source of truth
.agents/skills/               # procedures for agents and contributors, see engineering/workflow.md
.github/                      # CI workflows, issue forms, and pull request template
```

## Crate dependencies

> Status: `Decided`

The workspace follows a ports-and-adapters structure: `core` holds the domain logic and defines
traits (ports) for everything it needs from the outside world (storage, HTTP, clock, plugins,
platforms, rendering); other crates implement those traits; only the binary wires concrete
implementations together.

| Crate | May depend on (workspace crates) |
|---|---|
| `model` | none |
| `versions` | `model` |
| `core` | `model`, `versions` |
| `storage`, `plugin-host`, `declarative`, `conversation` | `core`, `model` |
| `web` | `core`, `conversation`, `model` |
| Native plugins in `plugins/` | `core`, `model`; platform plugins also `conversation` |
| `secure-notify` (binary) | any |
| `xtask` | `model` (schema generation) |

- `core` MUST NOT depend on adapter crates. Its public API exposes no SQL, HTTP-library, WASM, or
  platform types.
- WASM plugins depend only on `sdk/`.
- `model` and the other permissively licensed paths depend only on permissively licensed crates
  (see [../product/licensing.md](../product/licensing.md#permissively-licensed-parts)).
- CI checks the workspace dependency graph against this table (part of `cargo xtask ci`, see
  [../engineering/local-development.md](../engineering/local-development.md#developer-commands)).
  A workspace dependency the table does not allow requires changing the table first.

## Cross-cutting concerns

> Status: `Proposed`

- **Failure isolation:** a failing source, plugin, or platform MUST NOT affect others. Failures are
  logged, retried with backoff, and surfaced to admins (see
  [../operations/self-hosting.md](../operations/self-hosting.md)).
- **Untrusted input:** fetched content, uploaded files, and plugins are hostile by default. See
  [security.md](security.md).
- **Performance:** budgets and techniques are defined in [performance.md](performance.md).
- **Idempotency:** re-fetching the same item MUST NOT create duplicate events (unique
  `(source, source_item_id)`), and re-running the dispatcher MUST NOT send duplicates (delivery
  ledger).
- **Time:** all timestamps are stored in UTC; user-facing scheduling uses each subscriber's
  timezone.
