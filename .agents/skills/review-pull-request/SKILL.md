---
name: review-pull-request
description: Review a secure-notify pull request or diff, including agent-authored ones. Check it against the governing docs, verify that local verification evidence is real and sufficient, audit tests for slop with mutation checks, and apply the security, privacy, architecture, and performance rules. Use when asked to review a pull request or a set of changes.
---

# Review a pull request

**Use when:** reviewing a pull request or a diff, whoever (or whatever) wrote it.
**Not for:** preparing your own pull request ([open-pull-request](../open-pull-request/SKILL.md)).

## Read first

- The pull request description and diff.
- The "Read first" sections of the area skills that match the changed paths (see the
  [skills index](../README.md)). Load only those.
- [testing.md](../../../docs/engineering/testing.md) and
  [workflow.md: Local verification](../../../docs/engineering/workflow.md#local-verification).

## Procedure

1. **Scope.** One logical change? Correct base branch and Conventional Commit title
   ([git-workflow.md](../../../docs/engineering/git-workflow.md#pull-requests))?
2. **Docs.** Does the code do what the docs say? Were docs updated in the same pull request? Are new
   rules labeled `Proposed` and listed? Does anything contradict a `Decided` rule or depend on an
   `Open` question?
3. **Local verification evidence.** Present, specific, and plausible for the change type? Does it
   cover the failure path? Are unverified parts declared? Missing or vague evidence blocks approval.
4. **Tests.** For each new or changed test: which rule does it protect, and where does the expected
   value come from? Check the
   [forbidden patterns](../../../docs/engineering/testing.md#forbidden-test-patterns). For tests
   that protect critical rules, check out the branch, break the behavior, and confirm the test fails.
   A test does not earn trust from its pass status or its author.
5. **Architecture and code.** Crate dependency direction
   ([overview.md](../../../docs/architecture/overview.md#crate-dependencies)); no platform code
   outside platform plugins; errors, blocking work, and logging per
   [conventions.md](../../../docs/engineering/conventions.md#rust); user-facing text through i18n.
6. **Security and privacy** when the change touches untrusted input, personal data, secrets, or
   authentication: apply [security-sensitive-change](../security-sensitive-change/SKILL.md).
7. **Performance** when a hot path or budget is affected: numbers are present
   ([performance-sensitive-change](../performance-sensitive-change/SKILL.md)).
8. **Dependencies** are justified and permitted
   ([add-dependency](../add-dependency/SKILL.md)).
9. **Migrations** follow [migrations.md](../../../docs/engineering/migrations.md).

## Writing the review

- Blocking issues first. Each one gives the location (`path:line`), the rule it breaks (doc link),
  and a concrete fix.
- Non-blocking suggestions are labeled as such.
- Approve only when the evidence of local verification is sufficient and no blocking issue remains.
- Agents never approve, merge, or add a pull request to the merge queue on their own authority;
  they report their findings to the human.

## Done when

- [ ] Every changed area was checked against its docs.
- [ ] The verification evidence and the tests were actually examined, not assumed.
- [ ] Findings are actionable and linked to rules.
