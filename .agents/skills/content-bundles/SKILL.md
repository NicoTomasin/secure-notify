---
name: content-bundles
description: Change how secure-notify packages and ships content between releases. Covers the bundle format (tar.zst with bundle.toml), detached ed25519 signatures, embedded public keys and rotation, the registry index, CI publishing from main, and the instance's content updater with verification, validation, atomic apply, rollback, and operator settings. Use for content registry code and its CI jobs, not for adding feeds or taxonomy entries.
---

# Content bundles and registry

**Use when:** changing the bundle format, signing, the registry index, the updater, or the CI jobs
that build and publish bundles.
**Not for:** adding content ([declarative-source](../declarative-source/SKILL.md),
[taxonomy-entry](../taxonomy-entry/SKILL.md)).

## Read first

- [content-registry.md](../../../docs/architecture/content-registry.md) (all sections)
- [security.md: Threats and mitigations](../../../docs/architecture/security.md#threats-and-mitigations)
  (compromised registry or CI)
- [git-workflow.md: Branching model](../../../docs/engineering/git-workflow.md#branching-model)
  (content lands on `develop`)
- [releases.md: Versioning](../../../docs/engineering/releases.md#versioning)

## Procedure

1. **Verification order:** signature first, then `min_core_version` and `content_schema`, then each
   file's `sha256`, then schema validation of every file, then an atomic apply that keeps the
   previous bundle for rollback. Nothing from an unverified bundle is parsed beyond its signature
   check. Load [security-sensitive-change](../security-sensitive-change/SKILL.md).
2. **Keys:** the binary embeds the current and the next public key; rotation and revocation follow
   [Signing key custody](../../../docs/architecture/content-registry.md#signing-key-custody).
3. **`content_schema`** is bumped when the content file format changes incompatibly.
4. **Operator settings** (`content.updates`, `content.pin`, `content.auto_enable_sources`) keep their
   documented meaning; updates can always be disabled.
5. **Capability expansions** in updated WASM plugins wait for admin approval.
6. **Additional registries** cannot reuse IDs from the official registry; conflicts are rejected and
   reported to admins.
7. **CI:** bundles are built from `develop` at most once a day when content changed, and on demand
   by a maintainer; only files valid against the latest release's schemas are included; signing
   runs only in the protected environment, after an approval that shows the content diff
   ([ci](../ci/SKILL.md)).
8. **Tests:** tampered archive, wrong key, revoked key, too-new `min_core_version`, unknown
   `content_schema`, a schema-invalid file, an apply interrupted halfway (previous content intact),
   rollback; on the build side, a file that needs unreleased core features is left out.

## Verify

In a build with the `dev` feature, sign a locally built bundle with a throwaway development key
([local-development.md](../../../docs/engineering/local-development.md#development-only-features)),
serve it from a local registry, and check the update, the admin summary, the `notify` and `off`
modes, and rollback. Never use or request the official signing key.

## Done when

- [ ] Verification order and atomic apply hold, with tests for every rejection path.
- [ ] Settings, key handling, and CI publishing match the docs.
- [ ] The update and rollback were verified locally with a development key.
