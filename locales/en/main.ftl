# SPDX-License-Identifier: AGPL-3.0-or-later
# English: the reference locale (docs/architecture/delivery.md#localization-i18n).

language-name = English

## Command descriptions shown by /help

command-start = Join this instance with a code
command-help = List the commands you can use
command-about = License, version, and source code
command-upload = How to send a manifest or lock file
command-inventory = List, add, or remove inventory entries
command-watch = Watch a technology
command-unwatch = Stop watching something
command-watching = List what you watch
command-language = Show or change the bot language
command-invite = Create an invite code for a person (admins)
command-subscribers = List subscribers (admins)
command-remove = Remove a subscriber and their data (admins)

## /start

start-admin = You are now the admin of this instance.
start-member = Welcome! You joined this instance.
start-already-registered = You are already registered. Send /help to see the commands.
onboarding =
    What this bot stores about you: your chat ID on this platform, your language, what you watch, and the inventory you upload or add, reduced to component names and versions. Uploaded files are never stored, and the inventory is not grouped by project.
    The operator of this instance can read that data. Send /help to get started.

## /help

help-header = Commands you can use:
help-line = /{ $command } — { $description }

## /about

about-version = Version: { $version }
about-license = License: { $license }
about-source = Source code: { $url }

## /upload and uploads

upload-prompt = Send one of these files in this chat: { $files }. It is read once and never stored; only component names and versions are kept.
upload-done = Inventory updated from { $file }: { $added } new, { $refreshed } already known.
upload-technologies = Technologies detected, now watched: { $technologies }.
upload-unsupported = I cannot read { $file }. Supported files: { $files }.
upload-too-large = The file is larger than { $limit } MB.
upload-too-many = The file lists more than { $limit } components.
upload-timeout = Reading the file took too long. Try a smaller file.
upload-malformed = { $file } could not be read; it does not look like a valid file of its kind.
upload-bun-binary = Bun’s binary lockfile (bun.lockb) is not supported. Bun 1.2 and later write the text lockfile bun.lock; with older versions run “bun install --save-text-lockfile”. Then send bun.lock.

## /inventory

inventory-empty = Your inventory is empty. Send a lock file or use /inventory add <purl> <version>.
inventory-header = { $count ->
    [one] One entry:
   *[other] { $count } entries:
}
inventory-no-version = (no version)
inventory-more = …and { $count } more.
inventory-usage = Use /inventory add <purl> [version], /inventory remove <purl>[@version], or /inventory clear.
inventory-added = Added { $component } { $version }.
inventory-invalid = “{ $component }” is not a package URL such as pkg:npm/lodash.
inventory-removed = { $count ->
    [one] Removed one entry.
   *[other] Removed { $count } entries.
}
inventory-cleared = Your inventory was cleared ({ $count } entries removed).

## /watch, /unwatch, /watching

watch-usage = Usage: /watch <name>, for example /watch Node.js. You can watch: { $available }.
watch-added = You now watch { $name }.
watch-already = You already watch { $name }.
watch-unknown = “{ $query }” is not in the catalog. You can watch: { $available }.
unwatch-usage = Usage: /unwatch <name>, as listed by /watching.
unwatch-removed = You no longer watch { $name }.
unwatch-not-watched = You do not watch { $name }.
watching-none = You do not watch anything yet. Use /watch <name>.
watching-header = You watch:
watching-line = { $name } ({ $id }){ $origin ->
    [derived] {" "}· from your uploads
   *[manual] {""}
}

## /language

language-current = Current language: { $language }. Available: { $available }.
language-usage = Change it with /language <code>, for example /language es.
language-changed = Language changed to { $language }.
language-unknown = “{ $code }” is not an available language. Available: { $available }.

## /invite

invite-created = Invite code: { $code }
invite-instructions = The person sends /start { $code } to this bot in a private chat. The code works once and expires on { $expires }.

## /subscribers

subscribers-header = { $count ->
    [one] One subscriber:
   *[other] { $count } subscribers:
}
subscribers-line = #{ $id } · { $role } · { $language } · since { $since }{ $active ->
    [no] {" "}· inactive
   *[yes] {""}
}
role-admin = admin
role-member = member

## /remove

remove-usage = Usage: /remove <subscriber number>, as listed by /subscribers.
remove-done = Subscriber #{ $id } and their data were removed.
remove-not-found = There is no subscriber #{ $id }.
remove-self = You cannot remove yourself.

## Errors

error-permission-denied = Only admins can use /{ $command }.
error-unknown-command = Unknown command. Send /help to see the commands.
error-internal = Something went wrong. Please try again later.
