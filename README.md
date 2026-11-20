# secure-notify

An open-source, self-hosted notification bot that tells technical users (developers, sysadmins,
security teams) early and without noise about vulnerabilities, exploits, malicious packages,
breaches, attacks, and outages that affect what they actually use.

> **Status: design phase.** There is no application code yet. The design lives in [`docs/`](docs/README.md).

## What it will do

- Watch many sources: vulnerability databases, exploit signals, vendor and CERT advisories, breach
  and attack reports, status pages, security feeds.
- Normalize and deduplicate everything into stories, so each happening is notified once.
- Match stories against what each subscriber watches: components from uploaded manifests, lock
  files, and SBOMs; products, vendors, technologies, and online services; their own verified email
  addresses and domains.
- Deliver on the platforms subscribers choose (Telegram first), instantly or as hourly or daily
  digests, filtered by customizable rules and severity thresholds.
- Run on small hardware: a single Rust binary, one SQLite file, no inbound ports by default.
- Accept plugins written in any language that compiles to a WebAssembly component, and feeds added
  with a single TOML file.

## Documentation

- [Product vision](docs/product/vision.md)
- [Architecture overview](docs/architecture/overview.md)
- [Self-hosting](docs/operations/self-hosting.md)
- [Roadmap](docs/roadmap.md)
- [All docs](docs/README.md)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Contributors and coding agents should start with
[AGENTS.md](AGENTS.md).

## Security

Please report vulnerabilities privately as described in [SECURITY.md](SECURITY.md).

## License

secure-notify is licensed under the [GNU Affero General Public License v3.0 or later](LICENSE),
with an attribution requirement and a plugin interface exception described in [NOTICE](NOTICE).
Plugin SDKs, interface definitions, and schemas are licensed MIT OR Apache-2.0, so plugins can use
any license. Details: [docs/product/licensing.md](docs/product/licensing.md).
