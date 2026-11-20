---
name: wasm-plugin
description: Build a secure-notify plugin as a WebAssembly component in any language with component tooling. Pick the WIT world, use the guest SDK or bindings, rely only on host imports, exchange canonical JSON, respect resource limits, declare minimal capabilities, and test through the real host with plugin test. Load together with the skill for the plugin kind.
---

# WASM plugin

**Use when:** a source, parser, enricher, or exposure provider is built as a WASM component, in
Rust or any other language.
**Not for:** what the plugin does; load the kind skill too
([source-plugin](../source-plugin/SKILL.md), [parser-plugin](../parser-plugin/SKILL.md),
[enricher-plugin](../enricher-plugin/SKILL.md),
[exposure-monitoring](../exposure-monitoring/SKILL.md)). Host internals are
[plugin-runtimes-core](../plugin-runtimes-core/SKILL.md).

## Read first

- [plugin-runtimes.md: WASM runtime](../../../docs/architecture/plugin-runtimes.md#wasm-runtime)
- [plugins.md: Plugin manifest](../../../docs/architecture/plugins.md#plugin-manifest)
- [licensing.md: Plugin interface exception](../../../docs/product/licensing.md#plugin-interface-exception)

## Procedure

1. **Pick the world** for the kind (`secure-notify:plugin/<kind>@<major>`) and set the same value in
   `api` in `plugin.toml`.
2. **Language and bindings:** the Rust guest SDK in `sdk/rust/`; for other languages, the WIT
   bindings generator of that language's component tooling and the instructions in
   `sdk/<language>/` when present.
3. **Only host imports:** no WASI filesystem, sockets, environment variables, or subprocesses.
   Parsers get no network, state, or secret imports at all.
4. **Data crosses as canonical JSON** validated by the host; build it with the SDK's types where
   available.
5. **Within limits:** memory, CPU (fuel or epochs), wall-clock time, output size and item count.
   Use the host parse helpers for feeds, HTML, and JSON paths instead of bundling heavy parsers.
6. **Capabilities:** declare only the hosts, secrets, and state the plugin uses.
7. **Build:** document the exact build command and toolchain version in the plugin's `README.md`;
   CI rebuilds the component with that toolchain.
8. **License:** plugins in this repository are AGPL-3.0-or-later; external plugins that use only the
   published interfaces may use any license.
9. **Tests** run through `plugin test <id>`, which loads the component in the real host, so the
   boundary, capabilities, and limits are exercised too.

## Verify

- `plugin test <id>` with fixtures, then `--live` once (sources).
- Temporarily request an undeclared host or exceed a limit in a scratch build: only that call fails,
  with a clear error.

## Done when

- [ ] World, `api`, and capabilities match what the plugin uses.
- [ ] The component builds reproducibly from documented instructions.
- [ ] Fixture tests pass through the real host.
