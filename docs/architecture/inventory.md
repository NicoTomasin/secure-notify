# Inventory

A subscriber's inventory is a flat, anonymous set of **components** (packages identified by purl,
products identified by CPE), each with an exact version when known. It is built from uploaded
manifests, lock files, and SBOMs, and from manual additions. The inventory is deliberately
minimal: uploaded files are reduced to "component + version" pairs, so a database leak reveals
versions and technologies but not which project they belong to. Matching against the inventory
uses locally mirrored data, so the inventory never leaves the instance.

## Privacy

> Status: `Decided`

- Uploaded files MUST NOT be stored, in whole or in part. They are parsed in memory and discarded.
- Only the parsed result is stored, as flat **component + version** entries per subscriber, plus
  the derived watched technologies.
- No project name, file name, file hash, repository identity, or grouping of entries per upload is
  stored. Entries from all uploads are merged into one flat inventory.
- Uploading a file MUST NOT trigger follow-up questions (no "which project is this?").
- Threat model: a database leak reveals, per subscriber, which component versions and
  technologies they use, but not project names, file names, or which components belong together.
- Logs MUST NOT contain uploaded file contents or inventory entries.

## Inventory stays local

> Status: `Decided` (single-ecosystem mirror from the first start) / `Proposed` (rest)

- Inventory entries MUST NOT be sent to any third party, including vulnerability database APIs.
- Exact-version matching uses vulnerability data mirrored locally by sources (see
  [matching-and-rules.md](matching-and-rules.md)). Bulk databases are mirrored only for the
  ecosystems present in at least one inventory, then kept current incrementally. While an instance
  supports a single ecosystem for uploads (the MVP, see [../roadmap.md](../roadmap.md#phase-1-mvp)),
  that ecosystem is mirrored from the first start, so its data is ready before the first upload.

## Entries and maintenance

> Status: `Proposed`

- An entry is `(subscriber, component, version?, last_seen_at)`. `version` is empty when only a
  range is known (manifests such as `package.json`). Lock files and SBOMs give exact versions,
  including transitive dependencies. Direct and transitive dependencies are not distinguished.
- New uploads are merged as a **union**: new pairs are added, and pairs already present get their
  `last_seen_at` refreshed. Nothing is removed automatically, because without grouping the bot
  cannot know whether another project still uses an older version.
- Manual entries: `/inventory add <purl-or-product> <version>`, e.g.
  `/inventory add pkg:cargo/tokio 1.40.0` or `/inventory add product:fortinet/fortios 7.2.3`
  (product names are resolved through the taxonomy, see [taxonomy.md](taxonomy.md)).
- Stale entries are removed by the subscriber:
  - an inline **"I no longer use this version"** action on every alert that matched an inventory
    entry,
  - `/inventory` to list entries, `/inventory remove <component>[@version]`, and
    `/inventory clear`.
- Alerts mention when the matched entry was last seen in an upload if that was long ago (threshold
  configurable), so stale entries are easy to spot.

## Upload flow

> Status: `Proposed`

1. The subscriber sends a supported file (in groups: as a reply to a bot prompt, see
   [subscribers.md](subscribers.md)).
2. The core selects a parser by filename (see [plugins.md](plugins.md)), enforces size and
   complexity limits, and parses in memory.
3. Entries are merged into the inventory, derived technologies are added as watches, and the file
   is discarded.
4. The bot replies with a summary: detected technologies, number of components added or refreshed,
   and any unsupported content.
5. **Initial scan:** the new entries are checked once against the local vulnerability data,
   including backfilled stories, and a single report is sent. The stories in that report are
   recorded in the delivery ledger, so they are not sent again later as new.

### Initial scan before the first synchronization

> Status: `Decided`

The initial scan MUST NOT run against a mirror that has not completed its first synchronization,
because the stories that synchronization backfills are never notified as new (see
[deduplication.md](deduplication.md#backfill)) and the subscriber would never learn about them.

- When a mirror covering the new entries' ecosystem is still in its first synchronization, the
  summary reply says that vulnerability data is still synchronizing and that the report will follow.
- The initial scan for those entries runs when that synchronization completes, and its report is
  sent then.

## Planned file support

> Status: `Proposed`

| Ecosystem or format | Files |
|---|---|
| JavaScript | `package.json`, `package-lock.json`, `npm-shrinkwrap.json`, `pnpm-lock.yaml`, `yarn.lock`, `bun.lock` |
| Rust | `Cargo.toml`, `Cargo.lock` |
| Python | `pyproject.toml`, `poetry.lock`, `uv.lock`, `Pipfile.lock`, `requirements.txt` |
| PHP | `composer.json`, `composer.lock` |
| Go | `go.mod`, `go.sum` |
| Ruby | `Gemfile.lock` |
| Java/Kotlin | `pom.xml`, `gradle.lockfile` |
| .NET | `packages.lock.json`, `*.csproj` |
| Other ecosystems | `pubspec.lock` (Dart), `mix.lock` (Elixir), `Package.resolved` (Swift) |
| SBOM | CycloneDX (JSON, XML), SPDX (JSON, tag-value) |
| Containers and tools | `Dockerfile` (base image → OS and runtime), `docker-compose.yml` (service images such as postgres or redis), `.nvmrc`, `.tool-versions` |

The MVP supports the JavaScript formats for npm, pnpm, and Bun (see [../roadmap.md](../roadmap.md)).
Bun's binary lockfile (`bun.lockb`) is not parsed; the bot replies with how to produce the text
lockfile (`bun.lock`) instead. Other formats are added after the MVP.
