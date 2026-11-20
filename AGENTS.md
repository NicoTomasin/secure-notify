# AGENTS.md

Entry point for AI coding agents and human contributors working on **secure-notify**.
Read this file first, then load only what your current task needs (see "Loading context").

## What this project is

secure-notify is an open-source, non-profit, self-hosted notification bot written in Rust. Each
person or organization that runs it gets a private instance that watches security and reliability
sources (vulnerability databases, exploit signals, vendor advisories, breach and attack reports,
status pages, feeds, scraped pages) and notifies its subscribers, on the platforms they choose
(Telegram first), only about what affects the components, products, technologies, services, and
identities they watch. Subscribers describe their stack with bot commands or by uploading
manifests, lock files, and SBOMs (`package.json`, `Cargo.lock`, `composer.json`, CycloneDX, ...).
Plugins can be written in any language that compiles to a WebAssembly component.

Full product description: [docs/product/vision.md](docs/product/vision.md).

## Current phase

**Design.** There is no application code yet. The `docs/` folder is the source of truth for every
decision made so far, and `.agents/skills/` holds the procedures for doing the work. See
[docs/roadmap.md](docs/roadmap.md) for what comes next. Docs and skills mention tooling that does
not exist yet (`cargo xtask`, `plugin test`, the console platform); when a step needs a tool that is
missing, say so instead of improvising a substitute.

## Working with agents

AI-assisted and agent-driven work is fully accepted in this repository. The conditions are in
[docs/engineering/workflow.md](docs/engineering/workflow.md#ai-assisted-and-agent-contributions):
the submitter is responsible for the change, the changed flow MUST be verified locally before the
pull request is ready for review, and tests written by an agent are never accepted as proof that a
change works. Agents without access to Telegram or other platforms verify through the development
console ([docs/engineering/local-development.md](docs/engineering/local-development.md#console-platform)).

## Loading context

Context is a budget. Load in this order and stop as soon as you know enough:

1. This file (always).
2. The skills index, [.agents/skills/README.md](.agents/skills/README.md) (always; it is short).
3. Only the skills for the phase you are in and the area you are changing. Working on a scraper
   does not load platform skills; working on a bot command does not load source skills.
4. Only the doc sections those skills link to. Open other docs only when a link requires it; do not
   read `docs/` wholesale.

Drop a skill once you leave its phase or area. If no skill matches the task, use the table at the
end of this file to find the governing doc, and consider adding a skill (see the
[author-skill](.agents/skills/author-skill/SKILL.md) skill).

## Ground rules

1. **English only.** All code, comments, commit messages, docs, and agent instructions are written
   in English.
2. **Person- and agent-agnostic.** This is an open-source project. Docs, playbooks, and skills
   MUST NOT reference a specific person, account, machine, local path, or AI vendor/tool. Write for
   "the maintainer", "a contributor", or "the agent". The only exception is the canonical
   repository URL, which is written out only in [docs/product/licensing.md](docs/product/licensing.md)
   and [NOTICE](NOTICE).
3. **Docs are the source of truth.** Before designing or implementing anything, read the relevant
   docs (index: [docs/README.md](docs/README.md)). If code and docs disagree, do not pick one
   silently: follow the contradiction protocol below.
4. **Docs describe the current intended state only.** No changelogs, no "previously we did X",
   no deprecated sections. Git history is the archive.
5. **One rule, one place.** Each rule lives in exactly one doc. Other docs link to it instead of
   restating it.
6. **Keep docs in sync with work.** Any change that alters behavior, architecture, or a rule MUST
   update the affected docs in the same change.
7. **Docs hold rules, skills hold procedures.** Skills in `.agents/skills/` describe how to carry
   out a task and link to the doc sections that define the rules they apply. A skill MUST NOT
   introduce a rule of its own: new rules go into `docs/` first. How skills are organized is defined
   in [docs/engineering/workflow.md](docs/engineering/workflow.md#agent-skills).
8. **Tool entry files are pointers.** An agent tool that does not read this file natively MAY get a
   root-level entry file that only points to this file. Such files MUST NOT contain rules or
   instructions of their own.

## Task lifecycle

Every task moves through these phases (rules: [docs/engineering/workflow.md](docs/engineering/workflow.md#lifecycle-of-a-change)).
Load each skill when you reach its phase.

| Phase | Skill |
|---|---|
| Start: orient, scope, branch | [start-task](.agents/skills/start-task/SKILL.md), [git-workflow](.agents/skills/git-workflow/SKILL.md) |
| Change a doc or a rule | [change-docs](.agents/skills/change-docs/SKILL.md) |
| Implement | the area skills listed in the [skills index](.agents/skills/README.md) |
| Write or change tests | [write-tests](.agents/skills/write-tests/SKILL.md) |
| Verify locally | [verify-locally](.agents/skills/verify-locally/SKILL.md) |
| Open a pull request | [open-pull-request](.agents/skills/open-pull-request/SKILL.md) |
| Review a pull request | [review-pull-request](.agents/skills/review-pull-request/SKILL.md) |

## Contradiction protocol

When a request, a proposed feature, or existing code contradicts a rule documented in `docs/`:

1. **Stop** before implementing or documenting the conflicting change.
2. **Ask** the human you are working with. State the existing rule (file + section), the new
   requirement, and why they conflict. Offer the options (keep old rule / replace with new rule /
   alternative) and wait for an answer.
3. **If the new rule wins:** delete the old rule entirely, write the new rule in its place, update
   every doc that links to or depends on it, and mark it `Decided`. Do not leave the old text
   behind, struck through, or marked as deprecated.
4. **If the old rule wins:** adapt the change to comply with it.
5. Never resolve a contradiction silently, and never keep both rules.

Contributors without maintainer rights: a change to a `Decided` rule MUST be raised and approved
in an issue or pull request before it is merged (see [docs/community/governance.md](docs/community/governance.md)).

## Decision status labels

Every section that states rules carries one of these labels:

| Label | Meaning | How agents treat it |
|---|---|---|
| `Decided` | Explicitly confirmed by a maintainer. | Binding. Changing it requires the contradiction protocol. |
| `Proposed` | Current intended direction, not yet explicitly confirmed. | Build on it, but flag it to the maintainer when it materially shapes an implementation. Promote to `Decided` once confirmed, or replace it if rejected. |
| `Open` | Unresolved. Tracked in [docs/open-questions.md](docs/open-questions.md). | Do not implement. Ask first. |

When an open question is resolved, write the answer into the relevant doc and delete the
question from `open-questions.md`.

## Normative language

Docs use RFC 2119 keywords: **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, **MAY**.

## Where things are

| Topic | Doc |
|---|---|
| Docs index and conventions | [docs/README.md](docs/README.md) |
| Skills index (procedures) | [.agents/skills/README.md](.agents/skills/README.md) |
| Product vision, goals, non-goals, ethical boundaries | [docs/product/vision.md](docs/product/vision.md) |
| Event kinds and watch targets | [docs/product/scope.md](docs/product/scope.md) |
| Candidate sources | [docs/product/sources-catalog.md](docs/product/sources-catalog.md) |
| Glossary | [docs/product/glossary.md](docs/product/glossary.md) |
| License, permissive SDKs, attribution, repository URL, `/about` | [docs/product/licensing.md](docs/product/licensing.md) |
| System architecture, tech stack, repository layout, crate dependencies | [docs/architecture/overview.md](docs/architecture/overview.md) |
| Data model | [docs/architecture/data-model.md](docs/architecture/data-model.md) |
| Taxonomy (technologies, products, vendors, organizations) | [docs/architecture/taxonomy.md](docs/architecture/taxonomy.md) |
| Plugins (kinds, contracts, manifest) | [docs/architecture/plugins.md](docs/architecture/plugins.md) |
| Plugin runtimes (native, declarative, WASM) | [docs/architecture/plugin-runtimes.md](docs/architecture/plugin-runtimes.md) |
| Canonical feed and federation | [docs/architecture/feed-protocol.md](docs/architecture/feed-protocol.md) |
| Content bundles and registry | [docs/architecture/content-registry.md](docs/architecture/content-registry.md) |
| Subscribers, endpoints, access | [docs/architecture/subscribers.md](docs/architecture/subscribers.md) |
| Inventory and uploads | [docs/architecture/inventory.md](docs/architecture/inventory.md) |
| Exposure monitoring (emails, domains) | [docs/architecture/exposure-monitoring.md](docs/architecture/exposure-monitoring.md) |
| Deduplication and story correlation | [docs/architecture/deduplication.md](docs/architecture/deduplication.md) |
| Matching, severity, user rules | [docs/architecture/matching-and-rules.md](docs/architecture/matching-and-rules.md) |
| Delivery modes, routing, digests, i18n | [docs/architecture/delivery.md](docs/architecture/delivery.md) |
| Platforms and commands | [docs/architecture/platforms.md](docs/architecture/platforms.md) |
| Web panel and HTTP surfaces | [docs/architecture/web-panel.md](docs/architecture/web-panel.md) |
| Security and threat model | [docs/architecture/security.md](docs/architecture/security.md) |
| Performance budgets | [docs/architecture/performance.md](docs/architecture/performance.md) |
| Lifecycle of a change, local verification, AI-assisted work, skills | [docs/engineering/workflow.md](docs/engineering/workflow.md) |
| Gitflow, commits, pull requests, merging, merge queue | [docs/engineering/git-workflow.md](docs/engineering/git-workflow.md) |
| Testing policy | [docs/engineering/testing.md](docs/engineering/testing.md) |
| Local environment, developer commands, console platform | [docs/engineering/local-development.md](docs/engineering/local-development.md) |
| Database migrations | [docs/engineering/migrations.md](docs/engineering/migrations.md) |
| Settings, environment variables, and secrets in code | [docs/engineering/configuration.md](docs/engineering/configuration.md) |
| Engineering conventions (Rust, dependencies, logging, CI) | [docs/engineering/conventions.md](docs/engineering/conventions.md) |
| Releases and versioning | [docs/engineering/releases.md](docs/engineering/releases.md) |
| Self-hosting and instance config | [docs/operations/self-hosting.md](docs/operations/self-hosting.md) |
| Governance | [docs/community/governance.md](docs/community/governance.md) |
| Contributing | [docs/community/contributing.md](docs/community/contributing.md) |
| Roadmap | [docs/roadmap.md](docs/roadmap.md) |
| Open questions | [docs/open-questions.md](docs/open-questions.md) |
