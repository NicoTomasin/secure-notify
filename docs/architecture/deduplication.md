# Deduplication and story correlation

The same happening is usually reported by many sources at different times. secure-notify groups
events into **stories** and notifies subscribers about stories, never individual events. Before any
notification is dispatched, a double check against the delivery ledger guarantees a story is not
sent twice to the same subscriber, while still allowing explicit update notices when a story
changes materially.

## Requirements

> Status: `Decided`

- Keep a history of what was ingested and what was notified.
- If a subscriber was already notified about something because one source reported it first, later
  reports of the same thing from other sources MUST NOT trigger new notifications.
- A double check MUST run before dispatching any notification.

## Events vs stories

> Status: `Proposed`

- An **event** is one item from one source. Uniqueness: `(source, source_item_id)`. Re-fetching an
  existing item updates it only if its `content_hash` changed.
- A **story** is the real-world happening. Each event belongs to exactly one story.
- Matching, queueing, and delivery operate on stories. How a story aggregates its events is
  defined in [data-model.md](data-model.md).

## Kind families

> Status: `Proposed`

Events correlate only with stories of the same kind family (see
[../product/scope.md](../product/scope.md)). `news` events can join either family. When an event
of one family references a story of the other (e.g. a breach report naming the exploited CVE), the
two stories are linked as related, never merged.

## Correlation signals

> Status: `Proposed`

Applied in order; the first strong match wins.

1. **Identifiers (strong).** If the event shares any identifier with a story of a compatible
   family, it joins that story. Identifier aliases (e.g. OSV `aliases`) are added to the story so
   CVE ↔ GHSA ↔ ecosystem IDs resolve to the same story. Breach provider IDs and status page
   incident IDs are identifiers too.
2. **Linked identifiers (strong).** Identifiers found in the event's links or content (e.g. a blog
   post linking to `GHSA-xxxx-xxxx-xxxx`) are treated as the event's identifiers, subject to the
   family rule above.
3. **Canonical URL (strong).** URLs are normalized (lowercase host, no tracking parameters such as
   `utm_*`, no fragment, no trailing slash). The same canonical URL means the same story.
4. **Organization and time (weak, incident family).** Same organization entity, compatible kind,
   and close in time (default 72 h), plus title similarity above a threshold.
5. **Text similarity (weak).** Normalized-title similarity (trigram/Jaccard or MinHash) above a
   configurable threshold, plus at least one shared entity, within a time window (default 72 h).
   Deterministic only; no LLM (see [../product/vision.md](../product/vision.md)).

If no signal matches, a new story is created.

## Conservative merging

> Status: `Proposed`

A false merge can hide a distinct critical story, which is worse than an occasional duplicate.
Weak signals MUST therefore be tuned conservatively: when uncertain, create a new story.

## Story merges

> Status: `Proposed`

When a new event carries identifiers belonging to two different existing stories of the same
family, the stories are merged into one. The delivery ledger is merged too: if a subscriber
received either story, the merged story counts as delivered for that subscriber (subject to the
update-notice rules below).

## Backfill

> Status: `Proposed`

- A source's first synchronization (and any re-synchronization requested by an admin) is a
  **backfill**. Its events are stored and correlated normally but flagged `backfill`.
- A story whose events are all backfilled is never notified as new. It is used for initial scans
  after uploads, `/report`, and `/search` (see [platforms.md](platforms.md)).
- A backfilled story becomes notifiable only through update notices or when a non-backfill event
  joins it with a material change.

## Retention

> Status: `Decided` (one week by default, configurable, minimal record kept, mirrors exempt) / `Proposed` (details)

- Detailed data expires after the retention period: **one week by default**, configurable by the
  operator.
- After detailed data expires, a **minimal deduplication record** is kept so the requirements above
  still hold: the story's identifiers, canonical URLs, kind, title, severity snapshot, and the
  delivery ledger rows that reference it.
- Locally mirrored vulnerability databases (see [inventory.md](inventory.md)) do not expire: they
  stay synchronized with their source, and records are removed only when the source withdraws them.

Details:

| Data | Expires after | Setting |
|---|---|---|
| Events (payloads) from non-mirror sources | Retention period since last change (default 7 days) | `retention.events` |
| Story details (summary, affected, entities, relations) of stories not updated within the period | Retention period (default 7 days) | `retention.events` |
| Minimal deduplication records and their ledger rows | Default 1 year since the story's last change | `retention.dedup` |
| Mirrored database records | Never; synchronized with the source | — |
| Queue items | When sent or dropped | — |

- Sources that mirror a database declare it in their manifest (`mirror = true`).
- A compacted story that receives a new event is rebuilt from that event and keeps its identity, so
  ledger checks and update notices keep working.
- `/search`, `/report`, and the "Details" action show only what is still retained; mirrored data is
  always available for initial scans and `/report`.

## Grace window

> Status: `Proposed`

New or materially changed stories get `eligible_at = now + grace window` (default 5 minutes,
instance setting `correlation.grace_window`). Stories are not dispatched before `eligible_at`, giving parallel sources
time to arrive and correlate. This applies to `instant` delivery as well.

## Double check before dispatch

> Status: `Proposed`

1. **At enqueue:** a story is not enqueued for a subscriber if the ledger or the queue already holds
   the same `(subscriber, story, kind)` with an equivalent snapshot.
2. **At dispatch:** inside a transaction, immediately before sending, the dispatcher re-checks the
   ledger (resolving merged stories) and skips the item if it was already delivered. The ledger
   row is written in the same transaction that marks the queue item as sent.
3. If sending to every routed endpoint fails, the queue item is retried; the ledger row is only
   committed once at least one routed endpoint succeeded. Per-endpoint failures are retried through
   delivery attempts (see [delivery.md](delivery.md)).

## Update notices

> Status: `Proposed`

Deduplication MUST NOT swallow material changes. An already-delivered story triggers an
**update notice** when, compared to the last snapshot sent to that subscriber:

- its severity level increased,
- it became `exploited`,
- a public exploit became available (`exploit_available`),
- a fix became available,
- an outage was resolved,
- a breach expanded (record count at least doubled, or a more sensitive data class was added).

Subscribers can enable or disable each trigger (see [delivery.md](delivery.md)). Update notices are
labeled as updates, not new stories, and go through the same rules and double check.
