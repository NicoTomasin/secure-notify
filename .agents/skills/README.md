# Skills index

Skills are short procedures for one phase of work or one area of the codebase. This index is the
only file in this folder to read in full. Open a skill only when its row matches what you are doing
right now, and open nothing else from here. Rules live in [`docs/`](../../docs/README.md); skills
link to them ([AGENTS.md](../../AGENTS.md), ground rule 7). Organization and format:
[docs/engineering/workflow.md](../../docs/engineering/workflow.md#agent-skills).

How to pick:

1. **Phase:** load the skill for the phase you are in (table 1). Drop it when the phase ends.
2. **Area:** match the paths you will touch or the task (table 3). Load one area at a time; when the
   task moves to another area, drop the previous area skill.
3. **Trigger:** load a cross-cutting skill (table 2) only when its trigger applies to your change.

Never load a skill "just in case". If two skills seem to apply, read both descriptions and load the
narrower one first.

## 1. Phase skills

| Skill | Load when |
|---|---|
| [start-task](start-task/SKILL.md) | Beginning any task: orient, find the governing docs, check status labels, plan, branch. |
| [git-workflow](git-workflow/SKILL.md) | Creating, committing to, syncing, or merging a branch; resolving conflicts. |
| [change-docs](change-docs/SKILL.md) | Editing `docs/`, `AGENTS.md`, or root community files; changing a rule or status label; resolving an open question. |
| [write-tests](write-tests/SKILL.md) | Writing, changing, or reviewing tests, snapshots, or fixtures. |
| [verify-locally](verify-locally/SKILL.md) | The change is implemented and the pull request is about to be opened or marked ready. |
| [open-pull-request](open-pull-request/SKILL.md) | Preparing and opening a pull request. |
| [review-pull-request](review-pull-request/SKILL.md) | Reviewing a pull request or a diff, including agent-authored ones. |
| [release](release/SKILL.md) | Cutting a release or a hotfix (maintainers). |
| [author-skill](author-skill/SKILL.md) | Adding or changing a skill or this index. |

## 2. Cross-cutting skills

| Skill | Load when | Paths |
|---|---|---|
| [rust-code](rust-code/SKILL.md) | Writing or restructuring Rust code; creating a crate. | `crates/**`, `plugins/**/src/**`, `xtask/**` |
| [add-dependency](add-dependency/SKILL.md) | Adding, upgrading, or removing a dependency. | `Cargo.toml` dependency tables, `Cargo.lock` |
| [database-migration](database-migration/SKILL.md) | Any change to the SQLite schema. | `crates/storage/migrations/**` |
| [storage](storage/SKILL.md) | Repository-layer code: queries, transactions, the writer task. | `crates/storage/**` |
| [config-and-secrets](config-and-secrets/SKILL.md) | Adding or changing a setting, environment variable, or secret. | `.env.example`, configuration code in `crates/secure-notify/` |
| [canonical-schema](canonical-schema/SKILL.md) | Changing the normalized event, canonical types, JSON Schemas, WIT packages, or the feed shape. | `crates/model/**`, `schemas/**`, `sdk/wit/**` |
| [i18n](i18n/SKILL.md) | Adding or changing user-facing text, or a translation. | `locales/**` |
| [security-sensitive-change](security-sensitive-change/SKILL.md) | Touching untrusted input, personal data, secrets, authentication, the HTTP client, or plugin capabilities. | — |
| [performance-sensitive-change](performance-sensitive-change/SKILL.md) | Touching a hot path or anything with a budget (CPU, memory, startup, binary size). | `**/benches/**` |
| [ci](ci/SKILL.md) | Changing CI workflows. | `.github/workflows/**` |
| [deployment](deployment/SKILL.md) | Changing the container image, the compose example, release artifacts, or operator setup. | `Dockerfile`, `docker-compose.yml`, `docs/operations/**` |

## 3. Area skills

| Skill | Load when | Paths |
|---|---|---|
| [declarative-source](declarative-source/SKILL.md) | Adding or fixing a feed, JSON API, static HTML scraper, status page, or canonical feed subscription, with no code. | `content/feeds/**` |
| [source-plugin](source-plugin/SKILL.md) | Writing a source that needs code: scraper with logic, API client, bulk database mirror. | `plugins/sources/**` |
| [parser-plugin](parser-plugin/SKILL.md) | Supporting or fixing a manifest, lock file, or SBOM format. | `plugins/parsers/**` |
| [enricher-plugin](enricher-plugin/SKILL.md) | Adding data to stories after correlation (EPSS, exploit signals, tags). | `plugins/enrichers/**` |
| [wasm-plugin](wasm-plugin/SKILL.md) | Building any plugin as a WASM component, in any language. Load with the plugin-kind skill. | `sdk/<language>/**`, plugins with `runtime = "wasm"` |
| [plugin-runtimes-core](plugin-runtimes-core/SKILL.md) | Core work on the WASM host, capability enforcement, the declarative engine, the shared HTTP client, or source scheduling. | `crates/plugin-host/**`, `crates/declarative/**` |
| [taxonomy-entry](taxonomy-entry/SKILL.md) | Adding or fixing a technology, product, vendor, organization, or keyword rule. | `content/taxonomy/**`, `content/rules/**` |
| [correlation](correlation/SKILL.md) | Event-to-story correlation, merges, backfill, grace window, retention. | `crates/core/` (correlator, retention) |
| [matching](matching/SKILL.md) | Relevance, inverted indexes, mutes, rules, severity normalization. | `crates/core/` (matcher, rules) |
| [version-ranges](version-ranges/SKILL.md) | Version and range semantics for an ecosystem. | `crates/versions/**` |
| [delivery](delivery/SKILL.md) | Queue, dispatcher, digests, ledger, update notices, message model and rendering. | `crates/core/` (queue, dispatcher) |
| [bot-command](bot-command/SKILL.md) | Adding or changing a command, its permissions, or its replies. | `crates/conversation/**` |
| [platform-plugin](platform-plugin/SKILL.md) | Telegram or any other platform integration. | `plugins/platforms/**` |
| [inventory](inventory/SKILL.md) | Upload flow, inventory storage, manual entries, initial scans. | `crates/core/` (inventory) |
| [subscribers](subscribers/SKILL.md) | Pairing, invites, registration, endpoint linking, roles, group permissions. | `crates/core/` (subscribers) |
| [exposure-monitoring](exposure-monitoring/SKILL.md) | Watched identities, ownership verification, encryption, exposure providers. | `plugins/exposure/**`, `crates/core/` (identities) |
| [web-panel](web-panel/SKILL.md) | HTTP server, panel pages, sessions, platform webhooks, health, metrics. | `crates/web/**` |
| [federation](federation/SKILL.md) | Publishing or consuming the canonical feed. | `crates/web/` (feed), `secure-notify-feed` sources |
| [content-bundles](content-bundles/SKILL.md) | Bundle format, signing, registry, the instance's content updater. | `crates/core/` (content updater), bundle CI jobs |

## Examples

| Task | Load (in order, each when its phase arrives) | Not loaded |
|---|---|---|
| Add a vendor's RSS advisory feed | start-task, git-workflow, declarative-source (+ taxonomy-entry if the vendor entity is missing), verify-locally, open-pull-request | platform, delivery, storage skills |
| Write a scraper that needs logic, in Go | start-task, git-workflow, source-plugin, wasm-plugin, write-tests, verify-locally, open-pull-request | platform-plugin, bot-command |
| Fix message splitting on Telegram | start-task, git-workflow, platform-plugin, write-tests, verify-locally, open-pull-request | source and parser skills |
| Add a column to endpoints | start-task, git-workflow, change-docs, database-migration, storage, subscribers, write-tests, verify-locally, open-pull-request | plugin skills |
| Add an operator setting for the grace window | start-task, git-workflow, config-and-secrets, correlation, write-tests, verify-locally, open-pull-request | platform, parser skills |
