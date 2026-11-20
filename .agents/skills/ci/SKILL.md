---
name: ci
description: Change secure-notify's GitHub Actions workflows safely. Pin actions by commit SHA, keep token permissions minimal, keep secrets in protected environments and away from untrusted pull request code, map triggers to gitflow branches and tags, and keep every pull request check runnable locally. Use for any change under .github/workflows/.
---

# CI workflows

**Use when:** adding or changing a workflow under `.github/workflows/`.
**Not for:** release decisions ([release](../release/SKILL.md)) or container contents
([deployment](../deployment/SKILL.md)).

## Read first

- [conventions.md: Continuous integration](../../../docs/engineering/conventions.md#continuous-integration)
- [security.md: Supply chain](../../../docs/architecture/security.md#supply-chain)
- [releases.md: Integrity](../../../docs/engineering/releases.md#integrity) and
  [Release process](../../../docs/engineering/releases.md#release-process)
- [content-registry.md: Signing key custody](../../../docs/architecture/content-registry.md#signing-key-custody)
- [git-workflow.md: Branches](../../../docs/engineering/git-workflow.md#branches)

## Procedure

1. **Pin** every third-party action to a full commit SHA, with the version in a comment.
2. **Permissions:** `contents: read` at workflow level; grant more per job only where needed.
3. **Secrets** (release signing, content signing, provider keys for live checks) live in protected
   environments that require a maintainer's approval. Workflows triggered by pull requests from
   forks never get secrets and never run untrusted code in a privileged context.
4. **Triggers:**

   | Event | Jobs |
   |---|---|
   | Pull request into `develop` or `main` | Every check in the CI list |
   | Merge queue build for `develop` (`merge_group`) | The required pull request checks, on the queued result |
   | Push of a `v*` tag on `main` | Release builds, signing, SBOM, provenance, publishing |
   | Daily schedule or maintainer request, when content on `develop` changed | Content bundle build (only files valid for the latest release), signing after approval, publishing |
   | Schedule | Live source checks, memory budgets, mutation testing |

5. **Merge queue:** every workflow that provides a required check also triggers on `merge_group`;
   a required check that does not run there leaves the queue waiting forever
   ([git-workflow.md: Merge queue](../../../docs/engineering/git-workflow.md#merge-queue)).
6. **Local parity:** every pull request check must also run through `cargo xtask ci`
   ([local-development.md](../../../docs/engineering/local-development.md#developer-commands)). A new
   check gets its `xtask` counterpart in the same pull request.
7. **No network in test jobs:** live-data checks belong only to scheduled jobs
   ([testing.md: Determinism](../../../docs/engineering/testing.md#determinism)).
8. **Release jobs** never enable the `dev` feature.

## Verify

Run the workflow on your branch or fork and check that it passes, fails when it should (break one
check on purpose), and prints no secret values. For a required check, confirm it also reports on a
merge queue build.

## Done when

- [ ] Actions pinned, permissions minimal, secrets confined to protected environments.
- [ ] Triggers match gitflow; the check also runs locally through `xtask`.
- [ ] The workflow ran on the branch with the expected results.
