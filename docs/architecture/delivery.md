# Delivery

Stories that pass a subscriber's rules are queued with a due time computed from that subscriber's
delivery preferences, then sent either one by one or grouped into hourly or daily digests, to the
endpoints selected by routing. Delivery is always preceded by the double check described in
[deduplication.md](deduplication.md).

## Delivery modes

> Status: `Decided`

Subscribers MUST be able to choose how they receive notifications:

- `instant`: one message per story, as soon as it is eligible.
- `hourly`: a digest every hour.
- `daily@HH:MM`: a digest once a day at a subscriber-chosen time.

## Per-severity modes, routing, timezone, quiet hours

> Status: `Proposed`

- The mode is configured **per severity level**, plus `off` to discard a level entirely.
- Each level can be **routed** to a set of the subscriber's endpoints; the default is the primary
  endpoint.

  ```toml
  [delivery]
  timezone = "UTC"
  quiet_hours = { from = "23:00", to = "08:00", except = ["critical"] }

  [delivery.severity]
  critical = { mode = "instant", endpoints = ["primary", "work-email"] }
  high     = { mode = "hourly" }
  medium   = { mode = "daily@09:00" }
  low      = { mode = "off" }
  unknown  = { mode = "daily@09:00" }

  [delivery.updates]
  severity_increase = true
  exploited = true
  exploit_available = true
  fix_available = true
  outage_resolved = false
  breach_expanded = true
  ```

- Endpoints are referenced by their label (see [subscribers.md](subscribers.md)).
- Times are interpreted in the subscriber's timezone.
- During quiet hours, non-excepted items are postponed until quiet hours end.
- Stories flagged `exploited` are delivered with the `critical` mode and routing regardless of their
  level.

## Queue and dispatcher

> Status: `Proposed`

- When a story passes the rules for a subscriber, a queue item `(subscriber, story, kind, due_at)`
  is created. `due_at` is the later of the story's `eligible_at` (grace window) and the next slot
  of the subscriber's mode for that severity.
- If a queued, unsent story escalates, its queue item is re-evaluated (it may move to an earlier
  slot); it is not duplicated.
- The dispatcher runs every minute, takes due items, groups them per subscriber, runs the double
  check, renders, sends to every routed endpoint, and writes the delivery ledger.
- The ledger is per subscriber; each send to an endpoint is a delivery attempt. Failed attempts are
  retried with backoff per endpoint without re-sending to endpoints that already succeeded. The
  backoff starts at one minute and doubles with each failed attempt, up to one hour. When the
  platform reports that the endpoint is gone, the endpoint is deactivated (see
  [subscribers.md](subscribers.md#subscribers-and-endpoints)) instead of retried.
- Empty digests are never sent.

## Message content

> Status: `Proposed`

Every single-story message includes: severity (with `exploited` and `exploit available` badges),
kind, title, relevance reason (e.g. "you watch PHP", "in your inventory: lodash 4.17.20"),
canonical link, and the number of sources reporting it. Kind-specific content:

| Kind | Additional content |
|---|---|
| `vulnerability`, `exploit`, `malicious-package` | Matched inventory components with their versions and fixed versions; when each matched entry was last seen in an upload, if long ago; CVSS and EPSS when known. |
| `breach` | Organization, disclosure date, record count, exposed data classes; for `identity` relevance, which watched identity is affected. |
| `attack` | Organization, status, impact. |
| `outage` | Service, status, impact, start time (and resolution time for update notices). |
| `release`, `bug`, `news` | Affected entities and summary. |

A digest groups stories by severity, then by kind and entity, with one line per story and links.
Messages exceeding a platform's size limit are split (limits are platform capabilities, see
[platforms.md](platforms.md)).

## Localization (i18n)

> Status: `Decided` (bot messages are localized) / `Proposed` (details)

- All user-facing text (notifications, digests, command replies, buttons, web panel) MUST go
  through the i18n layer. Hard-coded user-facing strings are not allowed.
- Code, logs, docs, and plugin identifiers stay in English.
- Translations use Fluent, in contributable files under `locales/<lang>/`. English is the reference
  locale. Missing keys fall back to English.
- Each subscriber has a language. Default for persons: the platform's reported client language
  when a matching locale exists, otherwise English. Default for teams: English. It can be changed
  with `/language`.
- Content coming from sources (titles, summaries) is shown as published, not translated.

## Inline actions

> Status: `Proposed`

On platforms with buttons, messages carry actions:

- **Mute story**: no further notices for this story.
- **Mute entity / component**: creates a mute, optionally time-limited.
- **I no longer use this version**: removes the matched entry from the inventory (see
  [inventory.md](inventory.md)).
- **Details**: shows all sources, identifiers, affected ranges, and related stories.

Mute actions map to `/mutes add` and "I no longer use this version" maps to `/inventory remove`.
On platforms without buttons, the message lists those equivalent commands, and, when the web panel
is enabled, a link to the story in the panel. **Details** has no command equivalent and is offered
only as a button.
