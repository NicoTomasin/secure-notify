---
name: open-pull-request
description: Prepare and open a secure-notify pull request. Self-review the diff, confirm docs are in sync, target the right gitflow branch with a Conventional Commit title, and fill the template with local verification evidence, test notes, and any new Proposed rules. Use when a change is ready for review.
---

# Open a pull request

**Use when:** the change is implemented, tested, and verified locally.
**Not for:** reviewing someone else's pull request
([review-pull-request](../review-pull-request/SKILL.md)).

## Read first

- [git-workflow.md: Pull requests](../../../docs/engineering/git-workflow.md#pull-requests)
- [contributing.md: Pull request requirements](../../../docs/community/contributing.md#pull-request-requirements)
- [workflow.md: Definition of done](../../../docs/engineering/workflow.md#definition-of-done)
- The template: [.github/pull_request_template.md](../../../.github/pull_request_template.md)

## Procedure

1. **Self-review the diff** against the base (`git diff origin/develop...HEAD`, or `origin/main`
   for a hotfix). Remove debug code, commented-out code, scratch files, and unrelated changes. New
   files carry the SPDX header
   ([licensing.md](../../../docs/product/licensing.md#source-files-and-packages)).
2. **Docs in sync.** Every behavior, architecture, or rule change is reflected in `docs/`. Collect
   the new or changed rules and their labels for the description.
3. **Tests.** Each new or changed test has its protected behavior and mutation check written down
   ([write-tests](../write-tests/SKILL.md)).
4. **Local verification** is done and its evidence is written down, with the console transcript or
   script when you used one ([verify-locally](../verify-locally/SKILL.md)). If it is incomplete,
   open the pull request as a draft.
5. **Sync with the base** and run `cargo xtask ci` one last time.
6. **Title:** a Conventional Commit; it becomes the squash commit on `develop`.
   **Target:** `develop`. Only maintainers' `release/*` and `hotfix/*` branches target `main`.
7. **Fill every section of the template.** Add a justification for each new dependency and
   before/after numbers for performance-sensitive changes. Mentioning AI assistance (especially for
   tests and fixtures) helps reviewers.
8. **Open it**, for example:

   ```sh
   git push -u origin feature/<topic>
   gh pr create --base develop --title "feat(core): ..." --body-file <file> [--draft]
   ```

9. **During review:** answer each comment, push new commits instead of rewriting reviewed ones, and
   rerun the affected verification after substantive changes. Mark the pull request ready only when
   nothing in the verification section is missing.
10. **Merging:** after approval, a reviewer or maintainer merges through the merge queue (or, under
    the interim rule, once the branch is up to date with `develop`). If a check fails on top of the
    latest `develop`, update the branch from `develop`, fix the cause, and rerun the local
    verification of the areas involved
    ([git-workflow.md: Merge queue](../../../docs/engineering/git-workflow.md#merge-queue)).

## Done when

- [ ] The diff contains only this change, with docs updated.
- [ ] Title, base branch, and template are complete and correct.
- [ ] Evidence of local verification and test notes are in the description, without personal data.
