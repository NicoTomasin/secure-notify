# Git workflow

secure-notify uses gitflow: `main` holds what has been released, `develop` integrates the next
release, and short-lived branches carry each change. This doc defines the branch types, commit
messages, pull requests, merge strategies, the merge queue, protected branches, and what agents may
do with git.
What a release involves beyond branching is in [releases.md](releases.md).

## Branching model

> Status: `Decided`

- The repository follows gitflow: two long-lived branches, `main` and `develop`, plus supporting
  branches for features, bug fixes, releases, and hotfixes.
- Every contribution, code or content, targets `develop`. Only release and hotfix branches, run by
  maintainers, target `main`.
- Content reaches instances between releases through content bundles built from `develop` (see
  [../architecture/content-registry.md](../architecture/content-registry.md#bundles)), not through
  separate branches.

## Branches

> Status: `Proposed`

| Branch | Branches from | Merges into | Purpose |
|---|---|---|---|
| `main` | — | — | Released code. Every merge into it is an application release, tagged `vX.Y.Z`. |
| `develop` | — | — | Integration of the next release and source of content bundles. Default branch of the repository and target of every contribution. |
| `feature/<topic>` | `develop` | `develop` | New behavior, content, refactors, docs, tests, tooling. |
| `bugfix/<topic>` | `develop` | `develop` | Fixes for bugs that do not need an immediate release. |
| `release/<X.Y.Z>` | `develop` | `main`, then back into `develop` | Stabilizing a release: version bump, release notes, and fixes only. Maintainers only. |
| `hotfix/<X.Y.Z>` | `main` | `main`, then back into `develop` | Urgent fixes for the latest release, including security fixes. Maintainers only. |
| `backmerge/<X.Y.Z>` | `main`, right after a release or hotfix merge | `develop` | Carries the back-merge, so that `main` itself is never the head branch of a pull request (merged head branches are deleted automatically). Maintainers only. |

- `<topic>` is short, lowercase, and kebab-case, optionally prefixed with an issue number
  (`feature/123-osv-mirror`).
- Release candidates MAY be tagged on a release branch as `vX.Y.Z-rc.N`.

## Commits

> Status: `Proposed`

- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/):
  `<type>(<scope>)!: <summary>`. Release tooling uses them to generate release notes.
- Types: `feat`, `fix`, `docs`, `content`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`,
  `revert`. The optional scope is a crate, plugin, or area name (`core`, `storage`, `telegram`,
  `osv`, `taxonomy`, `agents`, ...).
- The summary is imperative, lowercase, without a trailing period, at most 72 characters. The body
  explains why the change is needed when it is not obvious.
- Breaking changes (configuration or data compatibility, plugin API, canonical feed, event schema,
  content format; see [releases.md](releases.md#versioning)) carry `!` and a `BREAKING CHANGE:`
  footer.
- Commits contain only files that belong to the change: no scratch files, logs, local databases,
  `.env` files, or unreviewed snapshot files.
- Commits never contain secrets or personal data. A secret committed by mistake is considered
  leaked and MUST be revoked, not only removed from history.

## Pull requests

> Status: `Proposed`

- One logical change per pull request.
- Pull requests from `feature/*` and `bugfix/*` target `develop`; pull requests from `release/*`
  and `hotfix/*` target `main`.
- The pull request title is a valid Conventional Commit, because it becomes the squash commit on
  `develop`.
- The description follows the repository's pull request template, including the local verification
  evidence (see [workflow.md](workflow.md#local-verification)) and the notes on new tests (see
  [testing.md](testing.md#mutation-check)).
- Contribution requirements and acceptance criteria are in
  [../community/contributing.md](../community/contributing.md).

## Merging

> Status: `Proposed`

| From | Into | Strategy |
|---|---|---|
| `feature/*`, `bugfix/*` | `develop` | Squash merge, through the [merge queue](#merge-queue). |
| `release/*`, `hotfix/*` | `main` | Merge commit (no squash, no rebase), so the back-merge is clean. |
| `backmerge/*` (equal to `main` after a release or hotfix) | `develop` | Merge commit, through a pull request opened right after the merge into `main`. |

- The history of `main` and `develop` is never rewritten.
- While a branch is private, it is kept current by rebasing onto its base. Once someone else builds
  on it, the base is merged into it instead. Force-pushing is allowed only to one's own branch,
  with `--force-with-lease`.
- When a branch is updated with new commits from its base, the tests and local verification of the
  areas touched by both are run again.

## Merge queue

> Status: `Decided` (`develop` merges through a merge queue) / `Proposed` (details and interim rule)

Many pull requests can pass CI on their own and still break `develop` together: two changes that
conflict semantically without a textual conflict, or two migrations with the same sequence number.
The merge queue prevents that by testing each pull request on top of everything merged before it.

- Pull requests into `develop` merge only through the merge queue. An approved pull request with
  passing checks is added to the queue; the queue builds it on top of the latest `develop` plus the
  pull requests ahead of it, runs the required checks on that result, and merges it only if they
  pass.
- The required checks are the pull request checks listed in
  [conventions.md](conventions.md#continuous-integration); benchmarks and scheduled jobs are not
  part of the queue.
- A pull request removed from the queue because a check failed goes back to its author, who updates
  the branch from `develop`, fixes the problem, and reruns the local verification of the areas the
  failure involves before it is queued again.
- Merges into `main` (release and hotfix branches) are rare and serialized by maintainers; they do
  not use the queue.
- **Interim rule:** the merge queue is not available to repositories owned by a personal account,
  which is the case of the canonical repository (see
  [../product/licensing.md](../product/licensing.md#repository-url)). Until the repository can use
  a merge queue, a pull request merges into `develop` only when its branch is up to date with
  `develop` and the required checks passed on that state. This gives the same guarantee one pull
  request at a time: when another pull request merges first, the author updates the branch and the
  checks run again.

## Protected branches

> Status: `Proposed`

These rules are enforced by repository rulesets, so they do not depend on anyone remembering them.

- `main` and `develop` are protected: no direct pushes, no force-pushes, no deletion; changes arrive
  only through pull requests with passing CI, and into `develop` only as described in
  [Merge queue](#merge-queue).
- Allowed merge methods follow [Merging](#merging): `develop` accepts squash merges and merge
  commits (for back-merges), `main` accepts merge commits only, and rebase merging is disabled.
  Squash and merge commits take the pull request title as their subject.
- Pull requests need the approval of a reviewer for the area or of a maintainer (see
  [../community/governance.md](../community/governance.md#roles)). A new push dismisses earlier
  approvals, and every review conversation is resolved before merging. While the project has a
  single maintainer, that maintainer MAY merge their own pull requests once CI passes; the rulesets
  let maintainers skip the approval only inside a pull request, never for a direct push.
- Required status checks are added to both rulesets, with "up to date before merging" for
  `develop`, together with the first CI workflow (see
  [conventions.md](conventions.md#continuous-integration)).
- Tags `v*` are created only by maintainers, on `main`, and are never moved or deleted.
- Merged head branches are deleted automatically.

## Rules for agents

> Status: `Proposed`

- Agents commit and push only to their own working branch, and only within the authority granted by
  the person operating them.
- Agents MUST NOT push to `main` or `develop`, create or move release tags, force-push branches they
  did not create, rewrite published history, bypass hooks or commit signing, merge pull requests, or
  add them to the merge queue.
- Exception: a maintainer MAY explicitly instruct an agent to merge specific pull requests or to tag
  a specific release. The agent then acts with that maintainer's authority, follows the same
  process (pull requests, merge methods, release steps), and says so in the pull request or
  release. Direct pushes to `main` and `develop` remain forbidden.
- Agents stage files explicitly and check the staged set before committing.
