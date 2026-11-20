# Development workflow

How every change moves from an idea to a merged pull request, for human contributors and coding
agents alike: the lifecycle of a change, the local verification required before review, what
"done" means, the conditions for AI-assisted work, and how agent skills are organized. Branching
mechanics are in [git-workflow.md](git-workflow.md); testing rules are in [testing.md](testing.md).

## AI-assisted and agent contributions

> Status: `Decided`

- Working with AI assistants and autonomous coding agents is fully accepted in this repository,
  for every kind of change.
- Every contribution, however it was produced, is held to the same standards, and the person who
  submits it is responsible for it.
- The one additional condition is that passing tests never prove a change works: the changed flow
  MUST be verified locally (see [Local verification](#local-verification)), and tests written by an
  agent are not accepted as evidence (see [testing.md](testing.md#principles)).

## Lifecycle of a change

> Status: `Proposed`

Each step names the skill that holds its procedure (see [Agent skills](#agent-skills)).

| Step | What happens | Skill |
|---|---|---|
| 1. Orient | Understand the request, find the governing docs, check status labels and open questions, apply the contradiction protocol. | [start-task](../../.agents/skills/start-task/SKILL.md) |
| 2. Branch | Create a branch from the right base. | [git-workflow](../../.agents/skills/git-workflow/SKILL.md) |
| 3. Design | When behavior, architecture, or a rule changes, update the docs first, on the same branch. | [change-docs](../../.agents/skills/change-docs/SKILL.md) |
| 4. Implement | Small, focused commits, following the skill for the area being changed. | see the [skills index](../../.agents/skills/README.md) |
| 5. Test | Tests that protect the documented behavior, each one mutation-checked. | [write-tests](../../.agents/skills/write-tests/SKILL.md) |
| 6. Verify locally | Run the changed flow on the real binary and record what was observed. | [verify-locally](../../.agents/skills/verify-locally/SKILL.md) |
| 7. Pull request | Open the pull request with the evidence. | [open-pull-request](../../.agents/skills/open-pull-request/SKILL.md) |
| 8. Review and merge | A reviewer checks the change against the docs, the evidence, and the tests; the approved pull request merges through the [merge queue](git-workflow.md#merge-queue). | [review-pull-request](../../.agents/skills/review-pull-request/SKILL.md) |

## Local verification

> Status: `Decided` (local verification before review is required) / `Proposed` (minimum per change type)

- Before a pull request is marked ready for review, its author MUST have run the changed flow
  locally, on the real binary or tool for that kind of change (not only its tests), and MUST
  describe in the pull request what was run and what was observed.
- Draft pull requests MAY be opened before local verification, to get early feedback.
- Agents without access to a real platform verify through the console platform, which covers every
  flow except platform-specific rendering and transport (see
  [local-development.md](local-development.md#console-platform)).
- An agent that cannot verify part of a change (for example, a change to a platform plugin without
  a test bot of that platform) MUST state exactly what was not verified and why. It MUST NOT claim verification it did not perform. The
  human submitter completes the missing verification before marking the pull request ready.
- Evidence MUST NOT contain secrets, tokens, chat IDs, email addresses, or other personal data.

Minimum verification per change type (how to do each is in the
[verify-locally](../../.agents/skills/verify-locally/SKILL.md) skill; the tools are described in
[local-development.md](local-development.md)):

| Change type | Minimum local verification |
|---|---|
| Docs or skills only | Every link resolves; the rendered Markdown was read. |
| Declarative source, source plugin | `plugin test <id>` against fixtures, and once live against the real source; produced events inspected; `source_item_id` stable across two runs. |
| Parser | Real files of the format uploaded through the console platform or a test bot; reply summary and inventory checked; file content stored nowhere. |
| Enricher, exposure provider | The instance runs with the plugin enabled and the expected fields or findings appear on real or ingested stories. |
| Taxonomy or keyword rules | `content verify` passes; stories from real sources or fixtures are tagged as expected, with no unexpected tags. |
| Core pipeline (correlation, matching, delivery) | The instance runs on a scratch data directory; the changed path is triggered with real sources or console ingestion (`:ingest`, `:advance`); database state and delivered messages inspected. |
| Bot command, conversation layer | The command is exercised on the console platform, as different users and in a group, including permission-denied paths. |
| Platform plugin | The flow is exercised on a real test bot of that platform. |
| Migration | Applied to a fresh database and to a copy of an existing development database; backup created; the affected feature works afterwards. |
| Configuration | The instance starts with the setting absent, valid, and invalid (clear startup error). |
| Web panel | The panel runs locally and the changed pages work in a browser, with and without JavaScript. |
| CI workflow | The workflow ran on the branch (or a fork) and produced the expected result. |
| Container or deployment | The image builds and runs with a fresh volume and with a volume from the previous image. |

## Definition of done

> Status: `Proposed`

A change is done when:

- its behavior matches the docs, and the docs were updated in the same change
  ([AGENTS.md](../../AGENTS.md), ground rule 6);
- new rules introduced by the change are labeled `Proposed` and listed in the pull request;
- its tests protect the changed behavior and were mutation-checked, or the pull request explains why
  no test applies (see [testing.md](testing.md));
- local verification was performed and recorded in the pull request;
- CI passes;
- no secret, personal data, scratch file, or local database is part of the change.

## Agent skills

> Status: `Proposed`

- Skills are short procedures stored in `.agents/skills/<name>/SKILL.md`, with YAML front matter
  holding `name` (equal to the folder name) and `description` (what the skill does and when to load
  it). This follows the open agent skills format, so tools that support skills can load them
  natively and other tools can read them as plain Markdown.
- The index [`.agents/skills/README.md`](../../.agents/skills/README.md) lists every skill with when
  to load it and the paths it covers. Agents read the index, then load only the skills that match
  their current phase and area.
- A skill has these sections: when to use it (and when not), what to read first (links to doc
  sections), the procedure, how to verify, and when it is done. Skills contain procedures only (see
  [AGENTS.md](../../AGENTS.md), ground rule 7) and stay agent-agnostic (ground rule 2).
- A skill stays under about 120 lines; a larger topic is split into several skills.
- A change to a doc section that a skill links to updates that skill in the same change.
- CI checks that every link in docs and skills resolves and that the index lists exactly the skill
  folders that exist (see [conventions.md](conventions.md#continuous-integration)).
