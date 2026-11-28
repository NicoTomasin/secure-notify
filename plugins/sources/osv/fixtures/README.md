# Fixtures: osv

Real OSV records of the npm ecosystem, retrieved on 2026-11-27 from the public bucket
`https://osv-vulnerabilities.storage.googleapis.com/npm/<ID>.json` (OSV data is licensed per
record, mostly CC-BY 4.0).

| File | Contents |
|---|---|
| `all.zip` | Six records, as `<ID>.json` entries, in the format of the bucket's `npm/all.zip`: GHSA-35jh-r3h4-6jhm, GHSA-xvch-5gv4-984h, GHSA-f82v-jwr5-mffw, GHSA-4j66-8f4r-3pjx, MAL-2026-20723, and the withdrawn GHSA-3966-f6p6-2qr9. Trimmed from 222 MB. |
| `modified_id.csv` | The bucket's `npm/modified_id.csv` reduced to the lines of those six records (their real `modified` times), newest first. |
| `entries/` | Single records served as `npm/<ID>.json` for incremental runs. |
| `index.json` | URL to file map for `plugin test osv`. |
