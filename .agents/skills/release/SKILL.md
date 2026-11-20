---
name: release
description: Cut a secure-notify application release or hotfix with gitflow (release/X.Y.Z or hotfix/X.Y.Z into main, tag vX.Y.Z, back-merge into develop), including the version bump, release notes, upgrade-path verification, and checking signed artifacts. Maintainers only; agents may prepare but never tag or publish.
---

# Release or hotfix

**Use when:** a maintainer cuts a release or a hotfix.
**Not for:** content-only updates; those ship as content bundles built from `develop`
([content-registry.md: Bundles](../../../docs/architecture/content-registry.md#bundles)).

## Read first

- [releases.md](../../../docs/engineering/releases.md) (all sections)
- [git-workflow.md: Branches](../../../docs/engineering/git-workflow.md#branches) and
  [Merging](../../../docs/engineering/git-workflow.md#merging)
- [migrations.md: Migration model](../../../docs/engineering/migrations.md#migration-model)

## Procedure: release

1. **Choose the version** per [Versioning](../../../docs/engineering/releases.md#versioning)
   (SemVer; `0.x` until the first stable release).
2. **Branch:** `git switch -c release/X.Y.Z origin/develop`. Bump `version` in the workspace
   `Cargo.toml`, update `Cargo.lock`, commit `chore(release): X.Y.Z`.
3. **Stabilize:** only fixes, docs, and release preparation land on the release branch.
4. **Release notes** from the Conventional Commits since the previous tag. Highlight breaking
   changes, new settings, migrations that older binaries cannot read, and plugin capability changes.
5. **Verify the upgrade path locally:** create a database with the previous release, upgrade it
   with the release candidate, check the automatic backup, the migration, startup, and one flow per
   major area. Build and run the container image with the previous image's volume.
6. **Refresh the upgrade fixture** for migration tests
   ([Testing migrations](../../../docs/engineering/migrations.md#testing-migrations)).
7. **Merge** the release pull request into `main` with a merge commit. Tag `main`:
   `git tag -a vX.Y.Z -m "vX.Y.Z"` and push the tag. The release workflow waits for a maintainer's
   approval in its protected environment.
8. **Back-merge:** open a pull request from `main` into `develop` and merge it with a merge commit.
9. **Check the published artifacts:** every Tier 1 target present, signatures verify, SBOM and
   provenance attached, container image tags correct
   ([Integrity](../../../docs/engineering/releases.md#integrity)).

## Procedure: hotfix

1. `git switch -c hotfix/X.Y.Z origin/main`; bump the patch version.
2. Fix with a regression test that fails before the fix ([write-tests](../write-tests/SKILL.md)).
3. Same verification, merge, tag, and back-merge as a release.
4. Security fixes: coordinate disclosure and the advisory per [SECURITY.md](../../../SECURITY.md);
   do not describe the vulnerability in public commits or pull requests before the advisory.

## Agents

Agents may prepare branches, version bumps, and release-note drafts. They MUST NOT push tags,
approve the release environment, or publish anything.

## Done when

- [ ] `main` carries the tag, `develop` contains the back-merge.
- [ ] Artifacts are published, signed, and verified; release notes are complete.
