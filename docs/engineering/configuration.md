# Configuration in code

How settings, environment variables, and secrets are added to secure-notify and handled in code.
The layers operators see (environment, instance config file, per-subscriber settings) are defined
in [../operations/self-hosting.md](../operations/self-hosting.md#configuration); this doc covers
how a new setting enters those layers safely.

## Choosing the layer

> Status: `Proposed`

| The value is | Layer |
|---|---|
| A secret: token, API key, password, encryption key | Environment variable or secret file. Never the instance config file or the database (see [../architecture/security.md](../architecture/security.md#secrets)). |
| Operator runtime information (paths, log level, source URL) | Environment variable. |
| An instance-wide behavior an operator tunes (sources, limits, registration, retention, HTTP surfaces) | Instance config file (`secure-notify.toml`). |
| A preference of one subscriber | The database, edited through bot commands or the web panel. |

## Naming

> Status: `Proposed`

- Environment variables are `UPPER_SNAKE_CASE` and name the thing, not the crate: platform tokens
  as `<PLATFORM>_BOT_TOKEN` or the platform's own term (`SMTP_URL`), provider keys as
  `<PROVIDER>_API_KEY`, plugin secrets exactly as declared in the plugin manifest.
- Every secret can also be read from a file: `<NAME>_FILE` holds the path of a file whose content is
  the value (container secrets). Setting both `<NAME>` and `<NAME>_FILE` is a startup error.
- Instance config keys are `snake_case`, grouped in tables named after the area that owns them
  (`registration`, `content`, `retention`, `http`, ...).

## Defaults and validation

> Status: `Proposed`

- Every setting has a default, except those the instance cannot run without (at least one platform,
  see [../operations/self-hosting.md](../operations/self-hosting.md#configuration)).
- Defaults never add inbound exposure, collect more data, or weaken a security control; features
  with those effects are off by default and enabled explicitly.
- Configuration is read once, at the binary boundary, into typed values that are passed to the
  code that needs them. Library crates do not read environment variables or files directly.
- All configuration is validated at startup. An invalid or unknown setting stops the instance with
  a message that names the setting and, for secrets, never shows the value.

## Secrets in code

> Status: `Proposed`

- Secrets are held in a type whose `Debug` and `Display` output is redacted, from the moment they
  are read until they are used.
- Secrets never appear in logs, error messages, panel pages, `/status` output, or crash reports.
- A plugin or source that needs a secret declares it in its manifest and stays disabled until the
  operator provides it (see [../architecture/plugins.md](../architecture/plugins.md#plugin-manifest)).

## Documenting settings

> Status: `Proposed`

A new or changed setting is documented in the same pull request:

- instance config keys in the doc of the area that owns them, next to their rules (as for
  `registration.*` in [../architecture/subscribers.md](../architecture/subscribers.md#joining-an-instance));
- environment variables in [../operations/self-hosting.md](../operations/self-hosting.md#configuration);
- every environment variable in `.env.example` (see
  [local-development.md](local-development.md#environment));
- the pull request description, so release notes mention it.
