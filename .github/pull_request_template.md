<!--
Title: a Conventional Commit, e.g. "feat(core): add grace window setting".
Target: develop. Only maintainers' release/* and hotfix/* branches target main.
Rules: docs/engineering/git-workflow.md and docs/engineering/workflow.md.
Never include tokens, chat IDs, email addresses, or other personal data in this description.
-->

## What and why

<!-- One short paragraph. Link the issue if there is one. -->

## Docs

- [ ] Docs are updated in this pull request for every behavior, architecture, or rule change, or
      there is no such change.

New or changed rules (doc section and status label; list every `Proposed` rule this pull request
introduces):

-

## Local verification

<!--
Required before marking this pull request ready for review: run the changed flow on the real
binary or tool, not only the tests. Minimum per change type:
docs/engineering/workflow.md#local-verification
-->

- Environment (OS, architecture):
- Steps or commands (console scripts can be attached so reviewers can rerun them):
- Observed:
- Failure path tried:
- Not verified, and why:

## Tests

<!--
For each new or changed test: the behavior it protects (doc section or issue) and how you broke
the code to see it fail. See docs/engineering/testing.md.
-->

| Test | Protects | Mutation check |
|---|---|---|
|  |  |  |

- [ ] Snapshot changes were reviewed line by line, or there are none.
- [ ] Fixtures are recorded or real, with provenance and without personal data, or there are none.
- [ ] No test was deleted, ignored, or weakened to make CI pass.

## AI assistance

<!--
Optional, and helpful for reviewers: which parts were produced with an AI assistant or agent,
especially tests and fixtures. AI-assisted work is welcome:
docs/engineering/workflow.md#ai-assisted-and-agent-contributions
-->

## Checklist

- [ ] One logical change; the title is a Conventional Commit; the base branch is correct.
- [ ] New files carry SPDX headers.
- [ ] New dependencies are justified below, or there are none.
- [ ] Security, privacy, and performance implications are described below, or there are none.
- [ ] All CI checks pass locally (`cargo xtask ci`).

## Notes for reviewers

<!-- Dependency justifications, benchmark numbers, migration notes, security-relevant aspects. -->
