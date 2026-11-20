---
name: add-dependency
description: Add, upgrade, or remove a dependency in secure-notify. Check that it is needed, well maintained, and license-compatible (permissive only for the SDK, schemas, and model), declare it once in the workspace with minimal features, keep OpenSSL out, pass cargo-deny, and justify it in the pull request. Use whenever Cargo.toml dependencies or a guest SDK's dependencies change.
---

# Add or change a dependency

**Use when:** a `Cargo.toml` dependency table changes, `Cargo.lock` changes beyond routine
updates, or a guest SDK in `sdk/` gains a dependency.
**Not for:** internal workspace dependencies; check those against the dependency table in
[rust-code](../rust-code/SKILL.md).

## Read first

- [conventions.md: Dependencies](../../../docs/engineering/conventions.md#dependencies)
- [licensing.md: Source files and packages](../../../docs/product/licensing.md#source-files-and-packages)
  and [Permissively licensed parts](../../../docs/product/licensing.md#permissively-licensed-parts)
- [security.md: Supply chain](../../../docs/architecture/security.md#supply-chain)
- [performance.md: Budgets](../../../docs/architecture/performance.md#budgets) (binary size)

## Procedure

1. **Question the need.** Can the standard library, an existing dependency, or a few dozen lines of
   our own code do it? Prefer that.
2. **Vet the candidate:** recent releases, active maintenance, wide use, no open unpatched
   advisories, a license compatible with AGPL-3.0-or-later. For `crates/model/`, `sdk/`, and
   `schemas/`, the dependency itself MUST be permissive.
3. **Declare it once** in `[workspace.dependencies]` with `default-features = false` and only the
   needed features; member crates use `<name> = { workspace = true }` and add features locally only
   when needed.
4. **TLS:** rustls only. `cargo tree -i openssl-sys` must find nothing.
5. **Checks:** `cargo deny check` passes; `cargo tree -d` shows no avoidable new duplicates.
6. **Size:** for a heavy dependency, compare the release binary size before and after against the
   budget.
7. **Justify** in the pull request: why it is needed, alternatives considered, license, maintenance
   signals, size impact.
8. **Upgrades:** read the changelog for breaking and security-relevant changes; run the full suite
   and the local verification of affected areas. An advisory affecting released code is fixed
   through a hotfix ([release](../release/SKILL.md)).
9. **Removal:** delete it from the workspace table and every member; confirm nothing else pulls it
   in unexpectedly.

## Done when

- [ ] The dependency is declared once, with minimal features, and passes cargo-deny.
- [ ] License constraints of the touched paths are respected; no OpenSSL.
- [ ] The pull request carries the justification and, if relevant, the size impact.
