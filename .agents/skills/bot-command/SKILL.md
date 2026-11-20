---
name: bot-command
description: Add or change a bot command in secure-notify's platform-agnostic conversation layer. Covers arguments, required permission (member, manager, admin), availability in private chats or groups, localized help and replies, inline-action equivalents, and calling core services through ports. Use for crates/conversation; platform-specific rendering belongs to platform-plugin.
---

# Bot command

**Use when:** adding a command, changing its arguments, permissions, availability, or replies.
**Not for:** how a platform renders or receives commands
([platform-plugin](../platform-plugin/SKILL.md)).

## Read first

- [platforms.md: Conversation layer](../../../docs/architecture/platforms.md#conversation-layer)
  and [Command set](../../../docs/architecture/platforms.md#command-set)
- [subscribers.md: Roles and permissions](../../../docs/architecture/subscribers.md#roles-and-permissions)
  and [Permissions in groups](../../../docs/architecture/subscribers.md#permissions-in-groups)
- [delivery.md: Localization (i18n)](../../../docs/architecture/delivery.md#localization-i18n)
- The doc of the feature the command manages (linked from the command set table).

## Procedure

1. **Docs first:** add or change the row in the
   [command set](../../../docs/architecture/platforms.md#command-set) and the feature doc
   ([change-docs](../change-docs/SKILL.md)).
2. **Define the command once** in the conversation layer: name, arguments, required permission,
   availability (private, group), help text keys.
3. **Permissions at execution time:** managers are checked through the platform API when the command
   runs; messages from unregistered chats are ignored, except `/start` with a valid code.
4. **Validate input** and apply per-subscriber rate limits; errors reply with a localized, actionable
   message.
5. **Effects go through core services** (ports), never through direct storage access from the
   conversation layer.
6. **Replies** use the message model and Fluent messages ([i18n](../i18n/SKILL.md)); no platform
   types or formatting in this crate.
7. **Buttons:** every inline action has a command or panel equivalent for platforms without buttons
   ([delivery.md: Inline actions](../../../docs/architecture/delivery.md#inline-actions)).
8. **Tests** with the fake platform: success path, permission denied (member in a team, non-admin),
   private versus group availability, invalid arguments; reply snapshots reviewed.

## Verify

In `dev console`: run the command as admin and as a member (`:as`), in a private chat and in a group
(`:private`, `:group`);
check every reply, the denied paths, and the resulting state (for example with `/watching` or
`/inventory`).

## Done when

- [ ] The command table and feature doc describe the command.
- [ ] Permissions, validation, and rate limits are enforced and tested.
- [ ] No platform-specific code; replies localized; verified on the console platform.
