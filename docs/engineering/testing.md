# Testing

How secure-notify is tested, and what is not accepted as a test. Tests exist to catch regressions
in documented behavior; a test that would still pass if that behavior broke is worse than no test,
because it creates false confidence and maintenance cost. This doc defines the test layers, how a
test is written and proven useful, the patterns that are rejected, and the rules for fixtures,
snapshots, determinism, and changing existing tests.

## Principles

> Status: `Decided`

- A test has value only if it fails when the behavior it protects breaks. Tests written to raise a
  count or a coverage number are not accepted.
- Passing tests never prove that a change works: every change is also verified locally (see
  [workflow.md](workflow.md#local-verification)).
- Tests written by an agent are not accepted as evidence that a change works. They are reviewed like
  any other code, with particular attention to what each assertion actually checks, and are kept
  only if they pass the [mutation check](#mutation-check).

## Test layers

> Status: `Proposed`

| Layer | Approach |
|---|---|
| Unit | Next to the code; pure functions preferred. |
| Plugins | Fixture tests for every source, parser, and enricher, in every runtime (see [../architecture/plugins.md](../architecture/plugins.md#contribution-and-quality) and [Fixtures](#fixtures)). |
| Version ranges | Property tests plus test vectors from real advisories. |
| Rendering | Snapshot tests per platform × locale for single messages and digests (see [Snapshots](#snapshots)). |
| Storage | Real SQLite (in memory or a temporary file) with the real migrations applied. |
| Migrations | Schema snapshot and upgrade tests (see [migrations.md](migrations.md#testing-migrations)). |
| Integration | Full pipeline with a fake platform plugin and mocked HTTP; deterministic clock. |
| Performance | Benchmarks and workload checks (see [../architecture/performance.md](../architecture/performance.md#measuring)). |

## Writing a test

> Status: `Proposed`

- Every test traces to a documented rule, a reported bug, or an adversarial case from
  [../architecture/security.md](../architecture/security.md#threats-and-mitigations). Its name states
  the behavior (`backfilled_story_is_not_notified_as_new`), not the function it calls.
- Expected values come from the docs, from real data, or from a hand-worked example, never from
  running the code under test or from a copy of its logic.
- A test asserts an observable outcome (returned value, persisted state, rendered or sent message)
  at the lowest layer that can observe it. Behavior that spans pipeline stages is tested at the
  integration layer.
- One behavior per test. Doc tables (severity mapping, default rules, relevance levels) become
  table-driven tests with one case per row.
- A bug fix starts with a regression test that fails before the fix.

## Forbidden test patterns

> Status: `Proposed`

| Pattern | Why it is rejected | Instead |
|---|---|---|
| Expected value computed by the code under test, or by a copy of its logic | Passes whatever the code does | Values from the docs, real data, or a hand-worked example |
| No assertion, or only `is_ok()`, `is_some()`, or "does not panic" | Checks nothing about the behavior | Assert the actual result |
| Testing the language or a dependency (derives, getters, constructors, serde round-trips of plain structs) | Tests the compiler or the library | Test project logic; test serialization only where custom formats or attributes matter |
| Mocking the unit under test, or asserting mainly on mock call counts | Tests the mock or the implementation | Fakes at the ports; assert outcomes |
| Mocking SQLite | Hides real SQL, constraints, and transactions | Real SQLite with the migrations applied |
| Invented fixtures that imitate a real source | Encode the author's assumptions instead of the source's behavior | Recorded fixtures (see [Fixtures](#fixtures)) |
| Many near-identical tests | Noise and maintenance cost | Table-driven or property tests |
| Large snapshots accepted without review | Freeze bugs as expected output | Small snapshots, reviewed line by line |
| Assertions on private internals or exact log lines | Break on refactors without catching regressions | Assert behavior; check logs only for "MUST NOT log" rules |
| Sleeps, wall-clock time, network, unseeded randomness | Flaky | See [Determinism](#determinism) |
| Expectations edited to match new output without a reason | Hide regressions | See [Changing existing tests](#changing-existing-tests) |

## Mutation check

> Status: `Proposed`

- Every new or changed test MUST be shown to fail when the behavior it protects is broken: break the
  behavior on purpose (invert a condition, remove the fix, change a constant), run the test, confirm
  it fails with a message that points to the problem, and restore the code.
- The pull request records, for each new or changed test, the behavior it protects and how the
  mutation check was done.
- A test that does not fail under the mutation check is fixed or deleted.
- For crates with pure logic (`versions`, matching, correlation, severity normalization), a
  scheduled CI job runs cargo-mutants; surviving mutants in changed code are reviewed.

## Fixtures

> Status: `Proposed`

- Fixtures are recorded from the real origin (`plugin record`, see
  [local-development.md](local-development.md#developer-commands)) or copied from public data, such
  as manifests and lock files from public open-source repositories.
- Each fixture directory has a `README.md` stating the provenance of every fixture: origin URL or
  repository and commit, and retrieval date.
- Synthetic fixtures are allowed only for cases real data cannot provide (oversized, malformed, or
  deeply nested input, boundary values) and are marked as synthetic in that `README.md`. Agents MUST
  NOT invent fixtures that imitate a real source or file format.
- Fixtures contain no secrets and no personal data. Values that identify people are replaced with
  reserved names (`example.com`, `example.org`, `.test` from RFC 2606) and documentation addresses
  (RFC 5737, RFC 3849), keeping the original structure.
- Fixtures are trimmed to the items the tests need.
- Declarative sources that map fields (`html`, `json-api`) SHOULD ship a recorded fixture so their
  mapping is tested offline.

## Snapshots

> Status: `Proposed`

- Snapshot tests are used for rendering (messages, digests, panel pages), not as a substitute for
  assertions elsewhere.
- Every new or changed snapshot is reviewed line by line before it is accepted; accepting snapshots
  in bulk without review is not allowed. Pending snapshot files are never committed.
- The pull request mentions which snapshots changed and why.

## Determinism

> Status: `Proposed`

- Tests MUST NOT reach the network; live-data checks of sources run only in a separate scheduled
  CI job.
- Time comes from an injected clock; timezones are explicit; randomness is seeded, and property-test
  regression files are committed.
- Tests do not sleep; asynchronous timing uses controllable time.
- Each test uses its own temporary directory and database, does not depend on test order, and does
  not read the developer's environment variables.

## Changing existing tests

> Status: `Proposed`

- When a test fails, the default assumption is that the code is wrong, not the test.
- Changing an expected value requires a reason in the commit message or pull request: the doc rule
  changed (with a link) or the test was wrong (with an explanation).
- Deleting, ignoring, or weakening a failing test to make CI pass is not allowed. A flaky test is
  fixed, or quarantined with a tracking issue and a maintainer's approval.

## Coverage

> Status: `Proposed`

Coverage percentages are not a goal and are not enforced in CI. Coverage reports MAY be used to find
documented rules that have no test.
