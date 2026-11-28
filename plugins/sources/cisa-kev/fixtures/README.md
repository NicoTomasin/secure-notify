# Fixture: cisa-kev

| File | Origin | Retrieved | Notes |
|---|---|---|---|
| `response-0.body` | `https://www.cisa.gov/sites/default/files/feeds/known_exploited_vulnerabilities.json` | 2026-11-27 | Trimmed to three of 1,808 entries (CVE-2026-120705, CVE-2021-21315, CVE-2020-29583) and reserialized with four-space indentation; `count` set to 3; no values changed otherwise. |

`index.json` maps the catalog URL to the body for `plugin test cisa-kev`.
