# Glossary

Shared vocabulary. Use these terms consistently in code and docs. Add a term here before using it
elsewhere.

## People, subscribers, and platforms

| Term | Definition |
|---|---|
| **Instance** | One running deployment of secure-notify, serving one or more subscribers. |
| **Operator** | Whoever deploys and runs an instance. Has access to its host, configuration, and database. |
| **Admin** | A person subscriber with instance-level permissions in the bot (invites, registration, source and plugin health). The first paired person becomes admin. |
| **User** | A person who uses an instance. Each user is represented by one person subscriber. |
| **Subscriber** | The owner of a configuration: watches, inventory, watched identities, rules, delivery preferences, timezone, and language. Kind `person` or `team`. Deduplication and the delivery ledger are per subscriber. |
| **Team** | A subscriber of kind `team`, representing a group of people. Its endpoints are group-type chats (a Telegram group, a Discord channel, a Slack channel, ...). |
| **Manager** | A person allowed to change a team's settings: a platform-level administrator of one of the team's group endpoints. |
| **Platform** | An external messaging or delivery service: Telegram, Discord, Matrix, Slack, email, webhook, ... |
| **Endpoint** | One destination on a platform (private chat, group chat, channel, email address, webhook URL), linked to exactly one subscriber. |
| **Primary endpoint** | The endpoint a subscriber's notifications go to unless routing says otherwise. |
| **Interactive endpoint** | An endpoint on a platform that can receive commands (e.g. Telegram). Delivery-only endpoints (email, webhook) cannot. |
| **Conversation layer** | The platform-agnostic part of the core that defines commands, permissions, and the message model once, for every platform plugin to render. |
| **Pairing code** | One-time code printed to the logs on first boot; the first person to send it becomes admin. |
| **Invite code** | Single-use, expiring code created by an admin that admits a new person or team. |
| **Link code** | Single-use, short-lived code that links an additional endpoint to an existing subscriber. |
| **Open registration** | Optional instance mode in which people can join without an invite, within admin-defined limits. |
| **Locale** | A Fluent translation bundle for one language. |
| **Web panel** | Optional, self-hosted web interface for editing subscriber settings and administering the instance. |

## Plugins and content

| Term | Definition |
|---|---|
| **Plugin** | A unit of functionality with a typed contract that connects the core to the outside world. |
| **Plugin kind** | What a plugin does: source, parser, enricher, exposure provider, or platform plugin. |
| **Plugin runtime** | How a plugin runs: native, declarative, or WASM. |
| **Native plugin** | A plugin written in Rust and compiled into the binary. |
| **Declarative source** | A source defined only by a TOML file (e.g. an RSS feed), with no code. |
| **WASM plugin** | A plugin compiled to a WebAssembly component from any language that supports the component model, run sandboxed inside the core. |
| **WIT** | WebAssembly Interface Types: the language used to define the contracts between the core and WASM plugins. |
| **Guest SDK** | A per-language library that helps authors build WASM plugins against the WIT interfaces. |
| **Plugin manifest** | The `plugin.toml` file describing a plugin: id, kind, runtime, API version, license, and capabilities. |
| **Capability** | A permission a plugin declares and the core enforces: allowed network hosts, named secrets, persistent state. |
| **Source** | A plugin that fetches items from an external origin and normalizes them into events. |
| **Parser** | A plugin that reads an uploaded manifest, lock file, or SBOM and extracts components and technologies. |
| **Enricher** | A plugin that adds data to stories after correlation (e.g. EPSS score, exploit availability). |
| **Exposure provider** | A plugin that checks watched identities against a breach data provider. |
| **Platform plugin** | A plugin that delivers messages to a platform and, optionally, receives commands from it. |
| **Content** | Community-maintained data shipped with secure-notify: declarative sources, taxonomy, keyword rules, and WASM plugins. |
| **Content bundle** | A signed, versioned archive of content. |
| **Content registry** | The location from which an instance downloads newer content bundles. |
| **Canonical feed** | The HTTP protocol through which an instance or any external system exposes normalized events for other instances to ingest. |
| **Federation** | One instance ingesting another instance's events through the canonical feed. |
| **SBOM** | Software Bill of Materials (CycloneDX or SPDX document) listing the components of a piece of software. |

## Events and stories

| Term | Definition |
|---|---|
| **Raw item** | Unprocessed data returned by a source's fetch step. |
| **Event** | A normalized item from a single source (e.g. one advisory entry, one blog post). |
| **Event kind** | The category of an event or story: `vulnerability`, `exploit`, `malicious-package`, `breach`, `attack`, `outage`, `release`, `bug`, or `news`. Defined in [scope.md](scope.md). |
| **Kind family** | A group of event kinds that can be correlated with each other: `vulnerability` family or `incident` family. Defined in [scope.md](scope.md). |
| **Story** | The real-world happening that one or more events describe (e.g. "CVE-2026-1234 in OpenSSL", "Data breach at Example Corp"). Subscribers are notified about stories, never about individual events. |
| **Identifier** | A strong, globally meaningful ID attached to an event or story: CVE, GHSA, OSV ID, ecosystem advisory ID, breach ID, status page incident ID. |
| **Correlation** | The process of attaching an event to an existing story or creating a new one. |
| **Grace window** | Delay after a story is created or changed before it becomes eligible for delivery, letting correlation settle. |
| **Backfill** | Historical events ingested during a source's initial synchronization. Backfilled stories are never notified as new. |

## Watching and matching

| Term | Definition |
|---|---|
| **Entity** | An entry in the taxonomy: a technology, product, vendor, or organization. |
| **Technology** | A language, framework, runtime, or tool (e.g. `tech:node`, `tech:postgresql`). |
| **Product** | A piece of software or hardware sold or distributed by a vendor, usually identified by CPE (e.g. `product:fortinet/fortios`). |
| **Vendor** | A company or project that publishes products (e.g. `vendor:fortinet`). |
| **Organization** | A company or online service that can suffer a breach, an attack, or an outage (e.g. `org:github`). |
| **Taxonomy** | The curated, contributable catalog of entities with their aliases, keywords, and identifier mappings. |
| **Watch** | A subscriber's declared interest in an entity. |
| **Component** | A package (identified by purl) or product (identified by CPE), optionally with an exact version, as recorded in an inventory. |
| **Inventory** | A subscriber's flat, anonymous set of components, merged from all their uploads and manual additions. It has no project names, file names, or grouping. |
| **purl** | Package URL, the standard package identifier format (`pkg:npm/lodash@4.17.21`). |
| **CPE** | Common Platform Enumeration, the standard product identifier used by NVD (`cpe:2.3:o:fortinet:fortios:7.2.3:*:*:*:*:*:*:*`). |
| **Watched identity** | An email address or domain that a subscriber has verified and asked to monitor for exposure in breaches. |
| **Exposure** | A finding that a watched identity appears in a known breach. |
| **Relevance** | How directly a story affects a subscriber: `identity`, `exact-version`, `component`, `product`, `organization`, or `technology`. |
| **Severity** | How serious a story is: `critical`, `high`, `medium`, `low`, or `unknown`. |
| **CVSS** | Common Vulnerability Scoring System, a 0–10 severity score published with many vulnerabilities. |
| **EPSS** | Exploit Prediction Scoring System, a daily probability (0–1) that a vulnerability will be exploited in the wild. |
| **KEV** | Known Exploited Vulnerabilities, the catalog of vulnerabilities confirmed as exploited, published by CISA. |
| **Exploited** | Flag indicating a vulnerability is known to be actively exploited in the wild. |
| **Exploit available** | Flag indicating a public exploit or proof of concept exists for a vulnerability. |
| **Rule** | A subscriber-defined filter combining relevance, kind, entity, component, source, severity, and flags. |
| **Mute** | A subscriber-defined suppression of a story, entity, component, or component version, optionally time-limited. Applied before rules. |

## Delivery

| Term | Definition |
|---|---|
| **Delivery mode** | When notifications of a given severity are sent: `instant`, `hourly`, `daily@HH:MM`, or `off`. |
| **Routing** | Which of a subscriber's endpoints receive notifications of a given severity. |
| **Digest** | A single message summarizing several stories, sent by `hourly` or `daily` modes. |
| **Update notice** | A follow-up notification about an already-delivered story whose situation changed materially (escalated severity, now exploited, fix available, ...). |
| **Delivery ledger** | The persistent record of which story (and which update) was delivered to which subscriber. |
| **Delivery attempt** | One try at sending a notification to one endpoint, recorded for retries and diagnostics. |

## Development

| Term | Definition |
|---|---|
| **Agent** | An AI coding assistant that reads the docs and skills and makes changes on behalf of the contributor operating it. |
| **Skill** | A short, task-scoped procedure in `.agents/skills/` that links to the docs holding the rules it applies. Defined in [../engineering/workflow.md](../engineering/workflow.md#agent-skills). |
| **Skills index** | `.agents/skills/README.md`: the table used to load only the skills that match the current phase of work and the area being changed. |
| **Gitflow** | The branching model of the repository: `main`, `develop`, and feature, bugfix, release, and hotfix branches. Defined in [../engineering/git-workflow.md](../engineering/git-workflow.md). |
| **Back-merge** | Merging `main` into `develop` after a release or a hotfix. |
| **Merge queue** | The mechanism that merges pull requests into `develop` one after another, each tested on top of everything merged before it. Defined in [../engineering/git-workflow.md](../engineering/git-workflow.md#merge-queue). |
| **Local verification** | Running the changed flow on the real binary on one's own machine before a pull request is ready for review. |
| **Mutation check** | Breaking on purpose the behavior a test protects, to confirm that the test fails. |
| **Fixture** | Recorded real input (a source response, an uploaded file) that tests run against. |
| **Console platform** | A development-only platform plugin that talks to a local instance through the terminal, interactively or from a script. Defined in [../engineering/local-development.md](../engineering/local-development.md#console-platform). |
| **Development clock** | The adjustable clock of a console instance, advanced with `:advance` to verify time-based behavior without waiting. |
| **Scratch data directory** | A disposable, git-ignored data directory used by a local instance. |
