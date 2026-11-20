---
name: plugin-runtimes-core
description: Work on secure-notify's plugin execution core. Covers the WASM component host and capability enforcement (crates/plugin-host), the declarative source engine (crates/declarative), the shared rate-limited HTTP client with SSRF protection, source scheduling, and failure isolation. Use for core runtime changes, not for writing individual plugins.
---

# Plugin runtimes (core)

**Use when:** changing how plugins are loaded, sandboxed, scheduled, or served host functions;
adding a declarative `type` or mapping feature; changing the shared HTTP client.
**Not for:** writing a plugin ([source-plugin](../source-plugin/SKILL.md),
[wasm-plugin](../wasm-plugin/SKILL.md)) or changing WIT contracts alone
([canonical-schema](../canonical-schema/SKILL.md)).

## Read first

- [plugin-runtimes.md](../../../docs/architecture/plugin-runtimes.md) (all sections)
- [plugins.md: Source context](../../../docs/architecture/plugins.md#source-context) and
  [Plugin manifest](../../../docs/architecture/plugins.md#plugin-manifest)
- [security.md: Threats and mitigations](../../../docs/architecture/security.md#threats-and-mitigations)
- [overview.md: Cross-cutting concerns](../../../docs/architecture/overview.md#cross-cutting-concerns)
  (failure isolation)
- [performance.md: Techniques](../../../docs/architecture/performance.md#techniques) (cheap WASM)

## Procedure

1. **Capabilities are the security boundary.** Every host import checks the manifest's declared
   hosts, secrets, and state; deny by default. Load
   [security-sensitive-change](../security-sensitive-change/SKILL.md).
2. **Limits:** memory, CPU (fuel or epochs), wall-clock timeouts, output size, and item counts are
   enforced per call and configurable by the operator. A violation fails only that call; repeated
   failures disable the plugin and notify admins.
3. **Shared HTTP client:** per-host rate limiting, conditional requests, retries with backoff,
   timeouts, response size limits, SSRF protection after redirects and DNS resolution, the generic
   User-Agent. It never carries subscriber or operator data.
4. **Declarative engine:** a new `type` or mapping feature is added to
   [plugin-runtimes.md](../../../docs/architecture/plugin-runtimes.md#declarative-runtime) first;
   update the declarative source JSON Schema ([canonical-schema](../canonical-schema/SKILL.md)) and,
   if the content file format changes incompatibly, `content_schema`
   ([content-bundles](../content-bundles/SKILL.md)).
5. **Failure isolation:** a failing plugin or source never blocks or crashes others; failures are
   logged, retried with backoff, and surfaced in `/status`.
6. **Performance:** ahead-of-time compilation cached in the data directory, instance pooling; measure
   ([performance-sensitive-change](../performance-sensitive-change/SKILL.md)).
7. **Tests:** hostile test components (infinite loop, memory blow-up, oversized output, undeclared
   host or secret); an SSRF matrix (private, loopback, link-local, metadata addresses, redirects to
   them); declarative mappings against recorded fixtures.

## Verify

- Run the built-in plugins through `plugin test <id>` and a scratch instance.
- Enable a deliberately failing plugin: other sources keep running, the plugin backs off, and admins
  are notified once per failure streak.

## Done when

- [ ] Capabilities and limits are enforced and tested with hostile components.
- [ ] Docs and schemas describe any new declarative feature.
- [ ] Failure isolation was verified on a running instance.
