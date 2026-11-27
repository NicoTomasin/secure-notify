# Releases

How secure-notify is versioned, built, signed, and distributed. The application, the plugin API,
the canonical feed protocol, the event schema, and the content format each have their own version,
so they can evolve at different speeds without surprising operators or plugin authors. Database
schema changes follow [migrations.md](migrations.md); branches follow
[git-workflow.md](git-workflow.md).

## Versioning

> Status: `Proposed`

| Artifact | Scheme | Compatibility promise |
|---|---|---|
| Application | SemVer; `0.x` until the first stable release | Within a major version: config and data stay compatible; upgrades are automatic. |
| Plugin API (WIT packages) | SemVer in the WIT package version | The host supports the current and the previous major version of each world. |
| Event schema | `schema_version` integer | Additive changes keep the version (see [../architecture/data-model.md](../architecture/data-model.md)). |
| Canonical feed | Version in the path (`/feed/v1`) | See [../architecture/feed-protocol.md](../architecture/feed-protocol.md). |
| Content | Bundle version and `content_schema` | See [../architecture/content-registry.md](../architecture/content-registry.md). |

## Artifacts

> Status: `Proposed`

| Tier | Targets |
|---|---|
| Tier 1 (tested in CI, supported) | Static Linux binaries (musl) for x86_64, aarch64, armv7; multi-arch container image |
| Tier 2 (built, best effort) | macOS (aarch64, x86_64), Windows (x86_64) |

- The container image is published on GitHub Container Registry; it runs as a non-root user, with a
  read-only root filesystem and a single data volume.
- Content bundles are released separately (see
  [../architecture/content-registry.md](../architecture/content-registry.md)).

## Integrity

> Status: `Decided` (`0.1.0` ships without signatures, SBOM, and provenance) / `Proposed` (rest)

- Every release artifact and image after `0.1.0`, the MVP release (see
  [../roadmap.md](../roadmap.md#phase-1-mvp)), is signed (keyless signing tied to the release
  workflow identity), with an SBOM (CycloneDX) and build provenance attestations.
- Release builds aim to be reproducible; differences are tracked as bugs.

## Release process

> Status: `Decided` (gitflow release and hotfix branches) / `Proposed` (details)

- A release is prepared on a `release/X.Y.Z` branch created from `develop`, where the workspace
  version is bumped and only fixes are accepted. It is merged into `main`, tagged `vX.Y.Z` on
  `main`, and `main` is then merged back into `develop` (see
  [git-workflow.md](git-workflow.md#branches)).
- A hotfix is prepared on a `hotfix/X.Y.Z` branch created from `main`, and is tagged and merged back
  the same way.
- Pushing a `vX.Y.Z` tag starts the release workflow, which runs in a protected environment that
  requires a maintainer's approval.
- Before a release is tagged, the upgrade path is verified locally: a database created by the
  previous release is upgraded by the release candidate (see [migrations.md](migrations.md)).
- Release notes list breaking changes, new settings, migrations that older binaries cannot read,
  and plugin capability changes.
- Release notes are generated from Conventional Commits and published on the repository's release
  page. They are not kept in `docs/` (docs describe only the current state, see
  [`../../AGENTS.md`](../../AGENTS.md)).
- Security fixes are released as soon as possible and announced through a security advisory (see
  [`../../SECURITY.md`](../../SECURITY.md)).
