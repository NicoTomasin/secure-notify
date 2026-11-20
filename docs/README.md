# Documentation

This folder is the source of truth for secure-notify's design. It is written to be consumed by
AI coding agents and humans alike: short sections, explicit rules, status labels, links instead of
duplication.

Working rules (language, contradiction protocol, status labels) are defined in
[../AGENTS.md](../AGENTS.md) and apply to every file here.

## Docs and skills

Docs hold the rules: what the system does and the constraints every change respects. Skills in
[`../.agents/skills/`](../.agents/skills/README.md) hold the procedures: how to carry out one
phase of work or a change in one area, linking back to the doc sections whose rules they apply.
How skills are organized is defined in [engineering/workflow.md](engineering/workflow.md#agent-skills).

## Layout

```
docs/
├── README.md                     # this index
├── product/
│   ├── vision.md                 # problem, goals, non-goals, ethical boundaries, principles
│   ├── scope.md                  # event kinds and watch targets (single definition)
│   ├── sources-catalog.md        # candidate sources per category
│   ├── glossary.md               # shared vocabulary
│   └── licensing.md              # AGPL-3.0-or-later, permissive SDKs, attribution, repository URL, /about
├── architecture/
│   ├── overview.md               # pipeline, process model, tech stack, repository layout
│   ├── data-model.md             # normalized event, story, persistence entities
│   ├── taxonomy.md               # technologies, products, vendors, organizations, tagging
│   ├── plugins.md                # plugin kinds, contracts, manifest, capabilities, quality
│   ├── plugin-runtimes.md        # native, declarative, and WASM runtimes
│   ├── feed-protocol.md          # canonical feed: federation and external systems
│   ├── content-registry.md       # signed content bundles and updates
│   ├── subscribers.md            # persons, teams, endpoints, linking, registration, permissions
│   ├── inventory.md              # components, uploads, privacy, file support
│   ├── exposure-monitoring.md    # watched emails and domains
│   ├── deduplication.md          # event → story correlation, backfill, double check, update notices
│   ├── matching-and-rules.md     # relevance, version evaluation, severity, rules
│   ├── delivery.md               # modes, routing, digests, message content, i18n
│   ├── platforms.md              # platform plugins, conversation layer, commands
│   ├── web-panel.md              # optional HTTP server: panel, feed, health, metrics
│   ├── security.md               # threat model, secrets, supply chain
│   └── performance.md            # target hardware, budgets, techniques, benchmarks
├── engineering/
│   ├── workflow.md               # lifecycle of a change, local verification, AI-assisted work, skills
│   ├── git-workflow.md           # gitflow branches, commits, pull requests, merge queue, agent git rules
│   ├── testing.md                # test layers, anti-slop rules, mutation check, fixtures, snapshots
│   ├── local-development.md      # toolchain, environment, developer commands, console platform
│   ├── migrations.md             # SQLite schema migrations: model, authoring, testing
│   ├── configuration.md          # settings, environment variables, and secrets in code
│   ├── conventions.md            # Rust standards, dependencies, logging, CI
│   └── releases.md               # versioning, artifacts, signing, release process
├── operations/
│   └── self-hosting.md           # deployment, configuration, first run, operations
├── community/
│   ├── governance.md             # roles, decision making, funding, code of conduct
│   └── contributing.md           # contribution paths, PR requirements, acceptance criteria
├── roadmap.md                    # Phase 0, MVP, capability tracks
└── open-questions.md             # unresolved decisions
```

## Writing conventions

- Each doc starts with a one-paragraph summary, then sections.
- Each section that states rules carries a status label: `Decided`, `Proposed`, or `Open`.
- Use RFC 2119 keywords for rules.
- Code snippets are illustrative contracts, not final implementations, unless stated otherwise.
- Use the terms defined in [product/glossary.md](product/glossary.md) consistently. Add new terms
  there before using them elsewhere.
- When adding a new doc, add it to the layout above and to the table in `AGENTS.md`.
- When renaming a heading, update every link to its anchor, including links from skills.
