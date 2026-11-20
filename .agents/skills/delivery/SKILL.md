---
name: delivery
description: Change how secure-notify queues, schedules, renders, and sends notifications. Covers per-severity delivery modes, routing, timezones and quiet hours, digests, the dispatcher, the double check, the delivery ledger and per-endpoint attempts, update notices, and the platform-neutral message model. Use for queue, dispatcher, and message rendering code in crates/core.
---

# Delivery

**Use when:** changing the queue, `due_at` computation, the dispatcher, digests, the ledger, update
notices, or the message model.
**Not for:** platform-specific rendering and transport ([platform-plugin](../platform-plugin/SKILL.md))
or whether a story is relevant ([matching](../matching/SKILL.md)).

## Read first

- [delivery.md](../../../docs/architecture/delivery.md) (all sections)
- [deduplication.md: Double check before dispatch](../../../docs/architecture/deduplication.md#double-check-before-dispatch),
  [Update notices](../../../docs/architecture/deduplication.md#update-notices),
  [Grace window](../../../docs/architecture/deduplication.md#grace-window)
- [platforms.md: Capabilities](../../../docs/architecture/platforms.md#capabilities)

## Procedure

1. **The ledger invariant is untouchable:** the double check runs at enqueue and at dispatch; the
   ledger row and the queue status change commit in one transaction; the ledger is committed only
   after at least one routed endpoint succeeded; retries never resend to endpoints that already
   succeeded.
2. **Time:** `due_at` is stored in UTC, computed from the subscriber's timezone, mode, quiet hours,
   and `eligible_at`. `exploited` stories use the `critical` mode and routing.
3. **Escalation** re-evaluates the existing queue item; it never creates a duplicate.
4. **Digests:** grouped by severity, then kind and entity; empty digests are never sent.
5. **Message model:** platform-neutral; every source-provided value is escaped by the renderer;
   messages split at the platform's `max_message_length`; all text through i18n
   ([i18n](../i18n/SKILL.md)).
6. **Update notices** fire only on the material changes the doc lists and the subscriber enabled,
   compared with the last snapshot sent to that subscriber.
7. **Tests** with an injected clock: restart between enqueue and dispatch (no duplicate); merged
   stories already delivered (no re-send); partial endpoint failure (retry only the failed one);
   quiet hours and timezone boundaries; digest grouping. Rendering snapshots per platform × locale,
   reviewed line by line.

## Verify

In `dev console`: `:ingest` an event matching the subscriber and `:advance` past the grace window.
Check one `instant` message, one ledger row, and nothing more after restarting the instance. Switch
the severity to an hourly or daily mode, ingest again, `:advance` to the next slot, and check the
digest. Ingest a material change and check the update notice is labeled as an update. A script
with `:expect` lines makes the run repeatable for reviewers.

## Done when

- [ ] The ledger invariant holds under restart, merge, and partial failure, with tests.
- [ ] Rendering goes through the message model and i18n; snapshots reviewed.
- [ ] Instant, digest, and update flows verified locally.
