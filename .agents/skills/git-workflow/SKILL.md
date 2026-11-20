---
name: git-workflow
description: Branch, commit, sync, and merge in secure-notify following gitflow (main, develop, feature/bugfix/release/hotfix branches) and Conventional Commits, including what agents may and may not do with git. Use whenever you create a branch, commit, update a branch from its base, or resolve conflicts.
---

# Git workflow

**Use when:** creating a branch, committing, keeping a branch current, resolving conflicts.
**Not for:** cutting a release ([release](../release/SKILL.md)) or opening the pull request
([open-pull-request](../open-pull-request/SKILL.md)).

## Read first

- [git-workflow.md](../../../docs/engineering/git-workflow.md) (short; read it all the first time)

## Procedure

1. **Choose the branch type** ([Branches](../../../docs/engineering/git-workflow.md#branches)):

   | You are doing | Base | Name |
   |---|---|---|
   | New behavior, content, refactor, docs, tests, tooling | `develop` | `feature/<topic>` |
   | Non-urgent bug fix | `develop` | `bugfix/<topic>` |
   | Urgent fix to the latest release | `main` | `hotfix/<X.Y.Z>` (maintainers) |

2. **Create it** from an up-to-date base:

   ```sh
   git fetch origin
   git switch -c feature/<topic> origin/develop
   ```

3. **Commit** in small logical steps:
   - stage explicit paths (`git add <paths>`), then read `git status` and `git diff --staged`;
   - nothing from `.local/`, no `.env`, logs, databases, pending snapshot files, or scratch files;
   - message per [Commits](../../../docs/engineering/git-workflow.md#commits):
     `feat(core): add grace window to story eligibility`.
4. **Keep the branch current:**
   - private branch: `git rebase origin/develop` (`origin/main` for a hotfix);
   - shared or under review: `git merge origin/develop`;
   - force-push only your own branch: `git push --force-with-lease`.
5. **Resolve conflicts** by keeping the intent of both sides. Then rerun the tests and the local
   verification of every area both sides touched. A migration number collision is resolved by
   renumbering yours (see [database-migration](../database-migration/SKILL.md)).
6. **Agents:** follow [Rules for agents](../../../docs/engineering/git-workflow.md#rules-for-agents).
   Push only your own working branch, only within the authority you were given; never push to
   `main` or `develop`, rewrite published history, or bypass hooks. Merge, queue, or tag only when a
   maintainer explicitly instructs it for that specific pull request or release.

## Done when

- [ ] The branch name and base match the table.
- [ ] Every commit is a valid Conventional Commit and contains only files of this change.
- [ ] The branch is current with its base and tests pass after the last sync.
