---
name: parser-plugin
description: Support or fix a manifest, lock file, or SBOM format in secure-notify. A parser turns uploaded file bytes into anonymous components (purl or CPE, optional version) and technologies, with no network, filesystem, state, or code execution, within size and complexity limits, and without storing or logging file content. Use for anything under plugins/parsers/ or file format support.
---

# Parser plugin

**Use when:** adding a supported upload format or fixing how one is parsed.
**Not for:** the upload flow, inventory merging, or initial scans ([inventory](../inventory/SKILL.md)).
For a WASM parser, also load [wasm-plugin](../wasm-plugin/SKILL.md).

## Read first

- [plugins.md: Parsers](../../../docs/architecture/plugins.md#parsers)
- [inventory.md: Privacy](../../../docs/architecture/inventory.md#privacy),
  [Entries and maintenance](../../../docs/architecture/inventory.md#entries-and-maintenance),
  [Planned file support](../../../docs/architecture/inventory.md#planned-file-support)
- [security.md: Threats and mitigations](../../../docs/architecture/security.md#threats-and-mitigations)
  (hostile uploads)
- [performance.md: Budgets](../../../docs/architecture/performance.md#budgets) (10 MB lock file)

## Procedure

1. **Check the plan** (planned file support, MVP scope in [roadmap.md](../../../docs/roadmap.md)).
   Detection is by filename; add `detect` only to disambiguate formats sharing a name.
2. **Output only components and technologies:** purl (or CPE) without version in the identifier,
   plus the exact version from lock files and SBOMs, transitive ones included; manifests with ranges
   produce entries without version. Derive technologies (for example `tech:node`).
3. **Never** emit project names, file names, paths, or other identifying metadata; never execute or
   evaluate file content (no running build scripts or manifests written in a programming language).
4. **Robustness:** stream large files; bound recursion depth; malformed input returns an error, never
   a panic. The core's size and complexity limits come first; do not rely on them alone.
5. **Version semantics** must match the ecosystem of the purl type
   ([version-ranges](../version-ranges/SKILL.md)).
6. **Fixtures:** real files from public open-source repositories, with repository and commit in the
   provenance `README.md`; adversarial synthetic files (huge, deeply nested, invalid encoding),
   marked as synthetic.
7. **Tests:** expected components checked by hand against the fixture, never produced by running the
   parser; one test per format variant (lockfile versions); adversarial files fail cleanly; a
   benchmark for the large-file budget.

## Verify

- In `dev console`: `:upload <path>` with real files of the format; check the reply summary,
  `/inventory`, and the initial scan report.
- Confirm the file content is nowhere in the data directory, database, or logs.
- Upload an unsupported variant and check the reply explains it.

## Done when

- [ ] Output is anonymous components and technologies only; nothing executed, stored, or logged.
- [ ] Real fixtures with provenance, adversarial cases, and the size budget are covered.
- [ ] Uploads were verified end to end on the console platform or a test bot.
