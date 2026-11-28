# Fixtures: OSV records for initial scans

Real OSV records of the npm ecosystem, retrieved on 2026-11-27 from the public bucket
`https://osv-vulnerabilities.storage.googleapis.com/npm/<ID>.json` (OSV data is licensed per
record, mostly CC-BY 4.0). They are complete and unchanged.

They affect packages of `plugins/parsers/npm/fixtures/npm-shrinkwrap-v2.json`, which pins
`follow-redirects` 1.14.9 and `form-data` 4.0.0:

| File | Affects |
|---|---|
| `GHSA-fjxv-7rqg-78g4.json` | `form-data` 4.0.0 (critical; fixed in 4.0.4). |
| `GHSA-jchw-25xp-jwwc.json` | `follow-redirects` 1.14.9 (moderate; fixed in 1.15.4). |
| `GHSA-74fj-2j2h-c42q.json` | Not 1.14.9: fixed in 1.14.7. |
