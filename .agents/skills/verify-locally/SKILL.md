---
name: verify-locally
description: Run the changed flow end to end on your own machine before a secure-notify pull request is marked ready for review, and record sanitized evidence. Agents without platform access use the dev console in script mode (users, groups, uploads, event ingestion, development clock). Required for every contribution; passing tests do not replace it. Use after implementing and before opening or readying a pull request.
---

# Verify locally

**Use when:** the change is implemented and tests pass, before the pull request is opened or
marked ready for review.
**Not for:** writing tests ([write-tests](../write-tests/SKILL.md)).

## Read first

- [workflow.md: Local verification](../../../docs/engineering/workflow.md#local-verification):
  the requirement and the minimum per change type.
- [local-development.md: Console platform](../../../docs/engineering/local-development.md#console-platform):
  directives, script mode, development clock.

## Procedure

1. **Find your change type** in the minimum-verification table and do at least that.
2. **Prepare a scratch environment:** `.env` copied from `.env.example`; `DATA_DIR` under
   `.local/`. Never use the token or database of an instance with real subscribers.
3. **Choose the front end:**
   - no platform access, or a change outside platform plugins: `dev console` (`--features dev`);
   - a change to a platform plugin: a test bot you created on that platform, optionally together
     with `dev console` to ingest events and advance time.
4. **Write a console script** for the flow (agents: always prefer script mode). Cover the exact
   path you changed, one neighboring path that could break, and the failure path (invalid input,
   permission denied, expired code). Use `:as` and `:group` for several users, `:upload` for files,
   `:ingest` for events copied from fixture output, `:advance` instead of waiting, and `:expect` to
   make each outcome checkable.
5. **Run it** (`cargo run --features dev -- dev console --script <file>`). A non-zero exit means a
   step failed or an error was logged: investigate, do not rerun until it passes by luck.
6. **Inspect what the transcript does not show:** database state (query the scratch database) and
   logs. Confirm the logs contain none of the data listed in
   [security.md: Logging](../../../docs/architecture/security.md#logging).
7. **Run every CI check locally** (`cargo xtask ci`).
8. **Write the evidence** for the pull request: environment (OS, architecture), the script or
   steps, the transcript or observations (short and sanitized), what you checked in the database
   and logs. No tokens, chat IDs, email addresses, or other personal data.
9. **State what you could not verify** and why. Agents MUST NOT claim verification they did not
   perform; the human submitter completes it before the pull request is marked ready.

## Hints per change type

| Change | How |
|---|---|
| Source | `plugin test <id>`, then `plugin test <id> --live` once; run twice and compare `source_item_id`s; inspect titles, links, dates, identifiers, kind, entities. |
| Parser | Console: `:upload <path>` with real files; check the reply, `/inventory`, and that the file content is nowhere in the data directory, database, or logs. |
| Pipeline | Console: `:ingest` events, `:advance` past the grace window; check stories, queue, ledger; restart the instance between enqueue and dispatch and confirm no duplicate message. |
| Digest, quiet hours, expirations | Console: set the mode or start the flow, then `:advance` to the slot or past the expiry. |
| Command | Console: as admin, as member, in a private chat and in a group; check permission-denied replies. |
| Platform plugin | Real test bot on that platform: pairing, a command, an upload if supported, a notification, a message long enough to split, removal from a group. |
| Migration | Fresh database, then a copy of your development database; `migrate`; backup file present; run a console script over the affected feature. |
| Container | Build the image; run with a fresh volume; restart; upgrade from the previous image's volume. |

## Done when

- [ ] The minimum verification for the change type was performed on the real binary or tool.
- [ ] Failure paths were exercised, preferably as `:expect` lines in a script.
- [ ] Sanitized evidence (script, transcript, observations) and any unverified parts are written
  down for the pull request.
