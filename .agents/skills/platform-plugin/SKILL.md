---
name: platform-plugin
description: Build or change a secure-notify platform integration (Telegram first; then email, webhook, Slack, Discord, WhatsApp, and later others). Native plugin, outbound-only connections, delivery plus optional interaction, accurate capability flags, native rendering with escaping and splitting, uploads, group managers, endpoint lifecycle, and operator onboarding docs. Use for anything under plugins/platforms/.
---

# Platform plugin

**Use when:** working on Telegram or any other platform integration.
**Not for:** command semantics ([bot-command](../bot-command/SKILL.md)) or what gets sent and when
([delivery](../delivery/SKILL.md)).

## Read first

- [platforms.md](../../../docs/architecture/platforms.md): sections
  [Platform plugins](../../../docs/architecture/platforms.md#platform-plugins),
  [Capabilities](../../../docs/architecture/platforms.md#capabilities),
  [Conversation layer](../../../docs/architecture/platforms.md#conversation-layer),
  [Platform order](../../../docs/architecture/platforms.md#platform-order),
  [Operator onboarding](../../../docs/architecture/platforms.md#operator-onboarding),
  [Planned platforms](../../../docs/architecture/platforms.md#planned-platforms) (your platform's row)
- [plugin-runtimes.md: Platform plugins and WASM](../../../docs/architecture/plugin-runtimes.md#platform-plugins-and-wasm)
- [subscribers.md: Subscribers and endpoints](../../../docs/architecture/subscribers.md#subscribers-and-endpoints)

## Procedure

1. **Order:** platforms follow the `Decided` platform order; starting one out of order needs a
   maintainer's agreement.
2. **Native crate** in `plugins/platforms/<id>/`, behind a cargo feature. Outbound connections only
   (long polling, WebSocket, outbound HTTP); inbound webhooks only through the optional HTTP server,
   with signature verification ([web-panel](../web-panel/SKILL.md)).
3. **Delivery first:** render the message model in the platform's format; escape every
   source-provided value; render only `http`/`https` links; split at `max_message_length`.
4. **Interaction (optional):** map native commands, buttons, and uploads to the conversation layer;
   enforce the upload size limit; check managers through the platform API; report lifecycle events
   (removed from a group, blocked) so endpoints are deactivated.
5. **Capabilities** declared exactly as the platform behaves.
6. **No platform code outside this plugin**
   ([plugins.md](../../../docs/architecture/plugins.md#platform-plugins)).
7. **Secrets:** the token comes from an environment variable
   ([config-and-secrets](../config-and-secrets/SKILL.md)); never logged; chat and user IDs appear in
   logs only as internal endpoint IDs.
8. **Operator docs:** onboarding steps and any costs for this platform.
9. **Tests:** rendering snapshots per locale; escaping of hostile titles (markup injection,
   `javascript:` links); splitting at the limit; a fake transport. Tests never call the real API.

## Verify

With a test bot you created on that platform: pairing with `/start <code>`, a command, an upload if
supported, a notification (run `dev console` with the platform enabled, `:ingest` an event, and
`:advance`), a long digest that must split, group behavior with a
manager and a non-manager, and removing the bot from a group (endpoint deactivated).

## Done when

- [ ] Outbound-only, capabilities accurate, all source content escaped.
- [ ] Operator onboarding documented; token never logged.
- [ ] The flows were verified on a real test bot of the platform.
