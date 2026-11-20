# Content registry

Declarative sources, the taxonomy, keyword rules, and WASM plugins change far more often than the
core. They are **content**: shipped inside every release, and also published as signed content
bundles that instances can download between releases, so new sources reach everyone without a
binary upgrade.

## Requirements

> Status: `Decided`

- Content ships with every release.
- Instances MAY additionally download newer signed content bundles from a content registry.
- The operator MUST be able to disable content updates.

## What content contains

> Status: `Proposed`

| Path | Contents |
|---|---|
| `content/feeds/` | Declarative sources. |
| `content/taxonomy/` | Technologies, products, vendors, organizations. |
| `content/rules/` | Keyword rules for tagging and severity. |
| `plugins/**` (WASM only) | Compiled WASM components and their manifests. |

Translations ship with the binary, not with content bundles, because their keys are tied to the
code.

## Bundles

> Status: `Decided` (bundles built from `develop`, containing only content the latest release accepts) / `Proposed` (format and details)

- A bundle is a `tar.zst` archive with a `bundle.toml` manifest:

  ```toml
  version = 2026112001            # monotonically increasing
  content_schema = 1              # format of the content files
  min_core_version = "0.4.0"      # oldest binary that can use this bundle

  [[files]]
  path = "feeds/nodejs-security.toml"
  sha256 = "…"
  ```

- Each bundle has a detached ed25519 signature (minisign format). The public keys that verify
  official bundles are embedded in the binary.
- CI builds bundles from the content on `develop`, where every contribution lands (see
  [../engineering/git-workflow.md](../engineering/git-workflow.md#branching-model)): at most once a
  day when content changed since the previous bundle, and on demand by a maintainer for urgent
  fixes.
- A bundle contains only the content files that validate against the schemas of the latest
  application release. Files that need unreleased core features are left out and ship with the next
  release. CI sets `content_schema` and `min_core_version` so that every binary the bundle admits
  accepts every file in it.
- Publishing a bundle requires a maintainer's approval in the protected signing environment (see
  [Signing key custody](#signing-key-custody)); the approval step shows the content diff since the
  previous bundle.
- CI publishes the bundle, its signature, and an index file as release assets of the canonical
  repository (see [../product/licensing.md](../product/licensing.md)). The default registry location
  is derived from the canonical repository URL.

## Instance behavior

> Status: `Proposed`

1. At startup, the instance uses the newest compatible content among the bundle embedded in the
   binary and the bundles installed in the data directory.
2. When updates are enabled, it checks the registry index on a schedule (default daily).
3. A new bundle is downloaded, its signature verified, `min_core_version` and `content_schema`
   checked, every file validated against its schema, and then applied atomically. The previous
   bundle is kept for rollback.
4. Admins receive a short summary of applied updates (new and changed sources, taxonomy changes).
5. New sources from an update are enabled only if marked `default_enabled = true` and the instance
   allows automatic enablement.
6. A WASM plugin update that expands its capabilities waits for admin approval (see
   [plugins.md](plugins.md)).

Operator settings (see [../operations/self-hosting.md](../operations/self-hosting.md)):

| Setting | Values |
|---|---|
| `content.updates` | `auto` (default), `notify` (download, apply on admin approval), `off` |
| `content.pin` | A bundle version to stay on |
| `content.auto_enable_sources` | `true` (default) or `false` |

## Additional registries

> Status: `Proposed`

- Operators MAY add registries (e.g. an organization's private sources) with their own public key.
- Content from additional registries MUST NOT reuse IDs from the official registry; conflicting
  items are rejected and reported to admins.
- Plugins from additional registries are external plugins and need explicit enablement (see
  [plugins.md](plugins.md)).

## Signing key custody

> Status: `Proposed`

- The official signing key is stored as a CI secret available only to a protected release
  environment that requires maintainer approval.
- The binary embeds the current and the next public key, so the key can be rotated with one
  release of overlap. A compromised key is revoked by releasing a binary without it.
