# Canonical feed protocol

The canonical feed is a small, read-only HTTP protocol for exposing normalized events. An instance
can publish its events through it, and another instance can ingest them as a source
(federation). Any external system, written in any language, can implement the same endpoint to
connect itself to secure-notify without writing a plugin. The protocol never carries subscriber
data.

## Requirements

> Status: `Decided`

- Federation is optional and disabled by default, both for publishing and for consuming.
- Federation is pull-based and read-only: consumers fetch, publishers never push.
- The feed MUST NOT expose any subscriber data: no subscribers, endpoints, watches, inventory,
  identities, rules, queue, or ledger. It exposes normalized events only.

## Endpoint

> Status: `Proposed`

```
GET {base}/feed/v1/events?cursor=<opaque>&limit=<1..1000>&kinds=<kind,...>
Authorization: Bearer <token>        (when the publisher requires it)
```

Response (`application/json`):

```json
{
  "schema_version": "1",
  "origin_instance": "3f6c2a9e-0b1d-4c55-9a51-2f0f7a0c8d11",
  "events": [
    {
      "via": ["3f6c2a9e-0b1d-4c55-9a51-2f0f7a0c8d11"],
      "origin_source": "cisa-kev",
      "...": "a normalized event as defined in data-model.md"
    }
  ],
  "next_cursor": "opaque-string",
  "has_more": false
}
```

- Events are ordered by the publisher's ingestion sequence. An event whose content changes is
  emitted again later in the sequence.
- `cursor` is opaque to consumers. Omitting it starts from the beginning of the publisher's
  retention.
- `kinds` optionally filters by event kind (see [../product/scope.md](../product/scope.md)).
- Publishers SHOULD support `ETag` / `If-None-Match` and MAY return `429` with `Retry-After`.
- The response is validated against the JSON Schema in `schemas/` (MIT OR Apache-2.0, see
  [../product/licensing.md](../product/licensing.md)).
- `origin_instance` is a random identifier generated on first boot; it identifies an instance, not
  its operator.

## Publishing

> Status: `Proposed`

- Enabled through the optional HTTP server (see [web-panel.md](web-panel.md)).
- Tokens are created by admins. A publisher MAY allow unauthenticated access explicitly
  (`public = true`).
- By default an instance publishes only events it ingested itself. Re-publishing events received
  through federation requires `republish = true`.
- Raw payloads are never published, only normalized events.

## Consuming

> Status: `Proposed`

- A consumer adds a declarative source with `type = "secure-notify-feed"` (see
  [plugin-runtimes.md](plugin-runtimes.md)):

  ```toml
  id = "community-hub"
  name = "Community instance feed"
  type = "secure-notify-feed"
  url = "https://hub.example.org"
  interval = "15m"
  secret = "COMMUNITY_HUB_TOKEN"
  ```

- The consumer keeps the cursor in its source state. Events are stored with `source` set to the
  feed's id and `source_item_id` set to `<origin_source>:<original source_item_id>`; correlation
  merges them with events the consumer ingests directly (see [deduplication.md](deduplication.md)).
- Loop prevention: consumers drop any event whose `via` list contains their own instance
  identifier; publishers append their own identifier to `via` when re-publishing.
- A consumer treats feed content as untrusted input like any other source (see
  [security.md](security.md)).

## Connecting external systems

> Status: `Proposed`

An external scraper, crawler, or internal tool can serve this endpoint from any language or
framework. It only needs to emit valid normalized events with stable `source_item_id` values and
honor the cursor semantics. This is the supported way to connect systems that do not fit the
plugin runtimes.

## Versioning

> Status: `Proposed`

The protocol version is in the path (`/feed/v1`). Additive changes stay in `v1`; breaking changes
create `v2`, and instances serve both during a transition announced in release notes (see
[../engineering/releases.md](../engineering/releases.md)).
