---
name: canonical-schema
description: Change secure-notify's canonical data contract, observed by plugins, declarative sources, and federated instances. Covers the normalized event and story types in crates/model, generated JSON Schemas in schemas/, WIT packages in sdk/wit, and the canonical feed shape. Classifies additive versus breaking changes and the version bumps they require. Use for any contract change.
---

# Canonical schema and plugin contract

**Use when:** changing types in `crates/model/`, files in `schemas/` or `sdk/wit/`, the event
kinds, or the shape of canonical feed responses.
**Not for:** internal types that never cross a plugin, feed, or content boundary
([rust-code](../rust-code/SKILL.md)).

## Read first

- [data-model.md: Canonical schema](../../../docs/architecture/data-model.md#canonical-schema) and
  [Normalized event](../../../docs/architecture/data-model.md#normalized-event)
- [releases.md: Versioning](../../../docs/engineering/releases.md#versioning)
- [plugin-runtimes.md: WASM runtime](../../../docs/architecture/plugin-runtimes.md#wasm-runtime)
- [feed-protocol.md: Versioning](../../../docs/architecture/feed-protocol.md#versioning)
- [licensing.md: Permissively licensed parts](../../../docs/product/licensing.md#permissively-licensed-parts)

## Procedure

1. **Classify the change.** Additive: a new optional field or a new enum value that consumers may
   ignore. Breaking: a removed, renamed, retyped, or newly required field, or changed semantics.
   Breaking changes bump `schema_version`, the WIT package major version, or the feed path version,
   per [Versioning](../../../docs/engineering/releases.md#versioning), and need a maintainer decision
   ([change-docs](../change-docs/SKILL.md)).
2. **Docs first:** update the field rules in
   [data-model.md](../../../docs/architecture/data-model.md#normalized-event), or the kinds in
   [scope.md](../../../docs/product/scope.md#event-kinds).
3. **Edit the Rust types** in `crates/model/`. The crate is MIT OR Apache-2.0: permissive
   dependencies only, matching SPDX headers.
4. **Regenerate the JSON Schemas** and commit the generated files; CI fails when they are stale.
5. **WIT:** bump the package version for breaking changes; the host keeps supporting the previous
   major version of each world.
6. **Update every producer and consumer:** normalizer validation, declarative engine defaults and
   mappings, built-in plugins and their fixtures' expected outputs, feed publisher and consumer,
   guest SDKs.
7. **Tests:** validation of real fixture events that use the new field; a compatibility test showing
   that events valid under the previous rules are still accepted (additive changes) or are handled
   as specified (breaking changes).

## Verify

- Run sources through `plugin test <id>` with fixtures and once live; inspect the events.
- If the feed shape changed, run two local instances and check a publish-consume round trip
  ([federation](../federation/SKILL.md)).

## Done when

- [ ] The change is classified, and any version bump follows releases.md.
- [ ] Docs, Rust types, generated schemas, WIT, and every producer and consumer agree.
- [ ] Compatibility is covered by tests on real events.
