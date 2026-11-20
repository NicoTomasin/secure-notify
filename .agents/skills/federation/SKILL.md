---
name: federation
description: Change secure-notify's canonical feed protocol. Covers publishing normalized events over /feed/v1 with opaque cursors, tokens, and ETags; consuming another instance's or an external system's feed as a secure-notify-feed declarative source; loop prevention with via; republishing; and protocol versioning. The feed never exposes subscriber data. Use for feed publisher code in crates/web or the secure-notify-feed source type.
---

# Federation and the canonical feed

**Use when:** changing how events are published through the feed, how a feed is consumed, or the
feed's shape or semantics.
**Not for:** other HTTP surfaces ([web-panel](../web-panel/SKILL.md)).

## Read first

- [feed-protocol.md](../../../docs/architecture/feed-protocol.md) (all sections)
- [web-panel.md: HTTP server](../../../docs/architecture/web-panel.md#http-server)
- [data-model.md: Canonical schema](../../../docs/architecture/data-model.md#canonical-schema)

## Procedure

1. **No subscriber data, ever:** the feed carries normalized events only; never subscribers,
   endpoints, watches, inventory, identities, rules, queue, ledger, or raw payloads. Build responses
   from an explicit allowlist of fields.
2. **Pull-only and read-only;** publishing and consuming are both off by default.
3. **Cursors** are opaque and stable; events are ordered by ingestion sequence; a changed event is
   emitted again later in the sequence.
4. **Loop prevention:** consumers drop events whose `via` contains their own instance identifier;
   publishers append theirs when republishing, and republish only with `republish = true`.
5. **Access:** admin-created tokens unless `public = true`; rate limits; `ETag` support; `429` with
   `Retry-After` when limiting.
6. **Consumers** store events with `source` set to the feed's id and `source_item_id` set to
   `<origin_source>:<original id>`, and treat feed content as untrusted input.
7. **Shape changes:** additive changes stay in `v1`; breaking ones create `v2`
   ([canonical-schema](../canonical-schema/SKILL.md)).
8. **Tests:** responses validate against the JSON Schema; a test asserts the payload contains only
   allowlisted fields; cursor resume after interruption; loop prevention with a `via` chain.

## Verify

Run two local instances with separate scratch data directories and ports. Instance A publishes;
instance B consumes A's feed as a `secure-notify-feed` source. Check that events reach B with the
expected `source` and `source_item_id`, correlate with B's own sources, and that A, consuming B in
turn, does not re-ingest its own events.

## Done when

- [ ] Only allowlisted event fields leave the instance, and a test proves it.
- [ ] Cursor, loop prevention, and versioning behave as documented.
- [ ] A two-instance round trip was verified locally.
