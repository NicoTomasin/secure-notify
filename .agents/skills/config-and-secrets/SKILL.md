---
name: config-and-secrets
description: Add or change an operator setting, environment variable, or secret in secure-notify. Choose the right layer, name it, give it a safe default, parse it once into typed config, validate at startup, keep secrets redacted and out of files and logs, and document it in the owning doc, self-hosting, and .env.example. Use when touching configuration parsing or any new tunable.
---

# Configuration, environment variables, and secrets

**Use when:** adding or changing an environment variable, an instance config key, or a secret, or
the code that reads them.
**Not for:** per-subscriber preferences edited through commands
([bot-command](../bot-command/SKILL.md)).

## Read first

- [configuration.md](../../../docs/engineering/configuration.md) (all sections)
- [self-hosting.md: Configuration](../../../docs/operations/self-hosting.md#configuration)
- [security.md: Secrets](../../../docs/architecture/security.md#secrets)

## Procedure

1. **Choose the layer**
   ([Choosing the layer](../../../docs/engineering/configuration.md#choosing-the-layer)): secrets
   and runtime values in the environment; instance-wide behavior in `secure-notify.toml`;
   subscriber preferences in the database.
2. **Name it** ([Naming](../../../docs/engineering/configuration.md#naming)): `UPPER_SNAKE_CASE`
   environment variables with a `<NAME>_FILE` variant for secrets; `snake_case` config keys in the
   owning area's table.
3. **Default:** safe and documented; nothing that adds inbound exposure, collects more data, or
   weakens a control is on by default.
4. **Parse once** at the binary boundary into typed values and pass them down. Library crates never
   read the environment or files themselves.
5. **Validate at startup:** an invalid or unknown setting stops the instance with a message naming
   the setting, never the secret's value.
6. **Secrets** live in a redacting type from read to use; never in TOML, the database, logs, errors,
   panel pages, or `/status` output. A plugin secret is declared in the manifest and keeps the
   plugin disabled until provided.
7. **Document** in the same pull request
   ([Documenting settings](../../../docs/engineering/configuration.md#documenting-settings)): the
   owning area doc, [self-hosting.md](../../../docs/operations/self-hosting.md#configuration) for
   environment variables, `.env.example`, and the pull request description.
8. **Tests** build configuration values explicitly and never read the developer's environment.
   Cover the default, a valid value, and an invalid value.

## Verify

Start the instance three times: setting absent (default applies), valid, invalid (clear error
naming the setting, no secret printed). For secrets, also try the `_FILE` variant and setting both
variants at once.

## Done when

- [ ] Right layer, conventional name, safe default.
- [ ] Parsed once, validated at startup, secrets redacted everywhere.
- [ ] Documented in the owning doc, self-hosting, and `.env.example`.
