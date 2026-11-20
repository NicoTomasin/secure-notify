# Product vision

secure-notify is an open-source, self-hosted, multi-platform notification bot (Telegram first) that
tells technical users (developers, sysadmins, security teams) early and without noise about
vulnerabilities, exploits, malicious packages, breaches, attacks, and outages that affect what they
actually use: their dependencies, products, technologies, online services, and identities. It
aggregates many sources, normalizes and deduplicates them into stories, and delivers them according
to each subscriber's rules and scheduling preferences, on the platforms they choose.

## Problem

Security and reliability information is scattered across vulnerability databases, exploit
trackers, vendor advisories, CERT bulletins, breach reports, status pages, and news sites.
Following all of it is noisy: most items are irrelevant to a given person or team, and the relevant
ones arrive repeatedly from several outlets. Existing dependency scanners work per repository and
focus on CVEs; breach monitors focus on personal accounts; status aggregators focus on uptime.
No open tool offers a single, private, customizable feed across all of a person's or team's
software, infrastructure, services, and identities.

## Goals

> Status: `Decided`

- **Technical audience.** The product targets developers, sysadmins, and security teams. UX
  assumes familiarity with packages, versions, and vendors.
- **Watch everything that matters.** Subscribers watch components (packages and products, with
  versions), technologies, vendors, organizations, and their own verified identities, and receive
  only relevant notifications. The full list is in [scope.md](scope.md).
- **Inventory from files.** Subscribers can describe their stack with commands and by
  **uploading manifest and lock files**, from which the bot derives components and technologies.
  Supported formats are listed in [../architecture/inventory.md](../architecture/inventory.md).
- **Many sources.** The bot ingests from structured databases, feeds, APIs, and scraped pages,
  aiming for complete coverage (see [scope.md](scope.md)).
- **Easy, polyglot contribution.** Adding a source MUST be easy. The community MUST be able to
  contribute plugins through pull requests, written in any language that compiles to a WebAssembly
  component, or with no code at all for feeds. See
  [../architecture/plugins.md](../architecture/plugins.md).
- **Normalized and deduplicated.** A story is notified once, no matter how many sources report
  it, verified by a double check before dispatch. See
  [../architecture/deduplication.md](../architecture/deduplication.md).
- **Customizable alerts.** Subscribers configure the minimum severity and fine-grained rules for
  what they receive. See [../architecture/matching-and-rules.md](../architecture/matching-and-rules.md).
- **Scheduling.** Subscribers choose how they receive notifications: one by one, hourly digest,
  or daily digest at a chosen time. See [../architecture/delivery.md](../architecture/delivery.md).
- **Multi-platform.** Notifications and commands work on several platforms (Telegram first), and
  one subscriber can link endpoints on several platforms. See
  [../architecture/platforms.md](../architecture/platforms.md).
- **Multi-subscriber instances.** One instance serves several people and teams, each with their own
  configuration. Instances are invite-only by default, with optional open registration. See
  [../architecture/subscribers.md](../architecture/subscribers.md).
- **Open source, self-hosted, non-profit.** Anyone can run a private instance with minimal
  configuration. Licensed AGPL-3.0-or-later with mandatory attribution. See
  [licensing.md](licensing.md) and [../operations/self-hosting.md](../operations/self-hosting.md).
- **Performance and low resource usage are first-class.** The core is written in Rust and MUST run
  well on small hardware. See [../architecture/performance.md](../architecture/performance.md).
- **Privacy by minimization.** Uploaded files are never stored; only anonymous component + version
  pairs are kept. Watched identities are opt-in, verified, and encrypted. See
  [../architecture/inventory.md](../architecture/inventory.md) and
  [../architecture/exposure-monitoring.md](../architecture/exposure-monitoring.md).
- **Optional federation.** Instances can share normalized events with each other. See
  [../architecture/feed-protocol.md](../architecture/feed-protocol.md).
- **Optional web panel.** Chat is the primary interface; a self-hosted web panel is optional. See
  [../architecture/web-panel.md](../architecture/web-panel.md).
- **Localized bot.** Bot messages support multiple languages. See
  [../architecture/delivery.md](../architecture/delivery.md).

## Non-goals

> Status: `Decided`

- **No LLM in the core.** Classification, correlation, and summarization MUST work with
  deterministic techniques. An optional LLM plugin may be reconsidered later; the core MUST never
  depend on one.
- **Not a hosted SaaS.** The project ships software and content for self-hosting; it does not
  operate a service for end users. Communities and organizations MAY run instances for their
  members.
- **Not a replacement for dependency update tooling.** The bot informs; it does not open pull
  requests or modify repositories.

## Ethical boundaries

> Status: `Proposed`

- secure-notify MUST NOT collect, store, or redistribute leaked data. It reports that a breach
  happened and which kinds of data were exposed, never the data itself.
- Sources MUST NOT connect directly to criminal leak sites, underground forums, or marketplaces.
  Breach and ransomware information comes from public reporting and public aggregators.
- No offensive capability: the project does not distribute exploit code and does not scan systems
  it does not own. It links to public references only.
- No telemetry. Instances never report usage to the project or anyone else.

## Principles

> Status: `Proposed`

- **Noise is the main failure mode.** When in doubt, prefer fewer, better notifications. Every
  feature is judged by whether it keeps the feed trustworthy.
- **Actionable over informative.** A notification SHOULD say what is affected (component and
  version in the subscriber's inventory, watched entity or identity) and how to respond (fixed
  version, vendor guidance) whenever that data exists.
- **Contributor-friendly.** The common contributions (a feed, a taxonomy entry) require no code;
  code contributions can be written in many languages.
- **Boring to operate.** One binary or container, one database file, outbound connections only by
  default.
- **Untrusted input everywhere.** Uploaded files, fetched content, and third-party plugins are
  treated as hostile. See [../architecture/security.md](../architecture/security.md).
- **Inventory never leaves the instance.** Version matching uses locally mirrored data, so a
  subscriber's components are never sent to third parties.
- **Privacy by default.** Store the minimum, log nothing personal, make deletion easy.
