---
name: write-tests
description: Write or review tests and fixtures that actually protect behavior in secure-notify. Derive tests from documented rules, take expected values from docs or real data, use recorded fixtures, and prove each test fails when the behavior breaks (mutation check). Use whenever you add, change, or review tests, snapshots, or fixtures, especially tests written by an agent.
---

# Write tests that are not slop

**Use when:** adding, changing, or reviewing tests, snapshots, or fixtures.
**Not for:** deciding whether the change works. That is
[verify-locally](../verify-locally/SKILL.md); tests never replace it.

## Read first

- [testing.md](../../../docs/engineering/testing.md) (all sections)

## Procedure

1. **List the behaviors to protect.** For each changed rule or fixed bug write one line:
   "given ..., when ..., then ...", with the doc section or issue it comes from. If you cannot name
   the rule a test protects, do not write that test.
2. **Choose the layer** ([Test layers](../../../docs/engineering/testing.md#test-layers)): the lowest
   one that can observe the outcome. Flows across pipeline stages go to the integration layer.
3. **Write the test from the rule, not from the code.** Expected values come from the doc, from real
   data, or from an example you worked out by hand. Never call the code under test, or copy its
   logic, to produce the expected value. Doc tables become table-driven tests, one case per row.
4. **Fixtures:** record them (`plugin record <id>`) or copy real public data, and write the
   provenance in the fixture folder's `README.md`. Synthetic input only for adversarial or boundary
   cases, marked as synthetic. Never invent content that imitates a real source. No personal data:
   use reserved names such as `example.com`
   ([Fixtures](../../../docs/engineering/testing.md#fixtures)).
5. **Run it.** It passes.
6. **Mutation check.** Break the behavior on purpose (invert a condition, remove the fix, change a
   constant) and run the test again. It MUST fail, with a message that points to the problem. Then
   restore the code. A test that still passes is fixed or deleted.
7. **Check against the forbidden patterns**
   ([table](../../../docs/engineering/testing.md#forbidden-test-patterns)): no computed
   expectations, no assertion-free tests, no tests of derives or the compiler, no mocked SQLite, no
   sleeps, no network.
8. **Snapshots:** read every line of each new or changed snapshot before accepting it; never accept
   in bulk; never commit pending snapshot files.
9. **Record for the pull request,** per new or changed test: the behavior it protects and how you
   broke the code for the mutation check.

## Reviewing tests written by an agent

- Treat them as untrusted, whatever their pass status. Read every assertion and ask where its
  expected value came from.
- Perform the mutation check yourself on the tests that protect critical rules (deduplication,
  privacy, version ranges, capabilities, escaping).
- Delete redundant and tautological tests instead of keeping them for their count.

## Never

- Delete, ignore, or weaken a failing test to make CI pass.
- Change an expected value without stating why in the commit or pull request
  ([Changing existing tests](../../../docs/engineering/testing.md#changing-existing-tests)).
- Assume the test is wrong when it fails; assume the code is wrong first.

## Done when

- [ ] Every new or changed test traces to a rule, a bug, or an adversarial case.
- [ ] Every new or changed test failed under a mutation check, recorded for the pull request.
- [ ] Fixtures have provenance and contain no personal data; snapshots were reviewed.
