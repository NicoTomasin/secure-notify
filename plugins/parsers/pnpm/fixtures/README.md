# Fixtures: pnpm parser

Real files from public open-source repositories, retrieved on 2026-11-27.

| File | Origin | Commit | Notes |
|---|---|---|---|
| `pnpm-lock-v5.yaml` | `Rich-Harris/estree-walker`, `pnpm-lock.yaml` | `8f7c1ebe458461ce9018d3f08464fb6fc0ac4bce` | Complete; lockfile version 5.4. |
| `pnpm-lock-v6.yaml` | `antfu/ni`, `pnpm-lock.yaml` | `3599d7ab24cbbed05cad445948558feba5b3496a` | Trimmed: the first six lines and three entries of `packages`, two with peer suffixes; lockfile version 6.0. |
| `pnpm-lock-v9.yaml` | `instructa/ai-prompts`, `pnpm-lock.yaml` | `a426026a0ab270f20e5707056375428d63071664` | Complete; lockfile version 9.0, with a pinned Node.js runtime entry. |

The YAML bomb and garbage inputs are synthetic, written inline in `tests/fixtures.rs`.
