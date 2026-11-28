// SPDX-License-Identifier: AGPL-3.0-or-later

//! The command registry: every command is defined once, with its permission, availability, and
//! localized help (docs/architecture/platforms.md#conversation-layer).

/// Who may run a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    /// Anyone, including unregistered chats (only `/start`).
    Anyone,
    /// Any registered subscriber.
    Subscriber,
    /// Admins only.
    Admin,
}

/// Where a command can be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// Private chats only.
    Private,
}

/// A command definition.
#[derive(Debug, Clone, Copy)]
pub struct CommandSpec {
    /// Name without the slash.
    pub name: &'static str,
    /// Required permission.
    pub permission: Permission,
    /// Where it can be used.
    pub availability: Availability,
    /// Fluent key of the one-line description shown by `/help`.
    pub description: &'static str,
}

/// Every command, in `/help` order.
pub const COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        name: "start",
        permission: Permission::Anyone,
        availability: Availability::Private,
        description: "command-start",
    },
    CommandSpec {
        name: "help",
        permission: Permission::Subscriber,
        availability: Availability::Private,
        description: "command-help",
    },
    CommandSpec {
        name: "about",
        permission: Permission::Subscriber,
        availability: Availability::Private,
        description: "command-about",
    },
    CommandSpec {
        name: "upload",
        permission: Permission::Subscriber,
        availability: Availability::Private,
        description: "command-upload",
    },
    CommandSpec {
        name: "inventory",
        permission: Permission::Subscriber,
        availability: Availability::Private,
        description: "command-inventory",
    },
    CommandSpec {
        name: "watch",
        permission: Permission::Subscriber,
        availability: Availability::Private,
        description: "command-watch",
    },
    CommandSpec {
        name: "unwatch",
        permission: Permission::Subscriber,
        availability: Availability::Private,
        description: "command-unwatch",
    },
    CommandSpec {
        name: "watching",
        permission: Permission::Subscriber,
        availability: Availability::Private,
        description: "command-watching",
    },
    CommandSpec {
        name: "rules",
        permission: Permission::Subscriber,
        availability: Availability::Private,
        description: "command-rules",
    },
    CommandSpec {
        name: "mutes",
        permission: Permission::Subscriber,
        availability: Availability::Private,
        description: "command-mutes",
    },
    CommandSpec {
        name: "language",
        permission: Permission::Subscriber,
        availability: Availability::Private,
        description: "command-language",
    },
    CommandSpec {
        name: "invite",
        permission: Permission::Admin,
        availability: Availability::Private,
        description: "command-invite",
    },
    CommandSpec {
        name: "status",
        permission: Permission::Admin,
        availability: Availability::Private,
        description: "command-status",
    },
    CommandSpec {
        name: "subscribers",
        permission: Permission::Admin,
        availability: Availability::Private,
        description: "command-subscribers",
    },
    CommandSpec {
        name: "remove",
        permission: Permission::Admin,
        availability: Availability::Private,
        description: "command-remove",
    },
];

/// The definition of command `name`.
pub fn find(name: &str) -> Option<&'static CommandSpec> {
    COMMANDS.iter().find(|c| c.name == name)
}

/// A parsed command line: `/name@bot arg1 arg2`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandLine<'a> {
    /// Lowercased name without slash or bot suffix.
    pub name: String,
    /// Whitespace-separated arguments.
    pub args: Vec<&'a str>,
}

/// Parses `text` as a command, if it starts with `/`.
pub fn parse(text: &str) -> Option<CommandLine<'_>> {
    let rest = text.trim().strip_prefix('/')?;
    let mut parts = rest.split_whitespace();
    let first = parts.next()?;
    // Telegram appends `@botname` in groups.
    let name = first
        .split('@')
        .next()
        .unwrap_or(first)
        .to_ascii_lowercase();
    if name.is_empty() {
        return None;
    }
    Some(CommandLine {
        name,
        args: parts.collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_are_parsed_with_bot_suffix_and_arguments() {
        let parsed = parse("/Start@secure_notify_bot  ABC123 ").unwrap();
        assert_eq!(parsed.name, "start");
        assert_eq!(parsed.args, ["ABC123"]);
        assert_eq!(parse("hello"), None);
        assert_eq!(parse("/"), None);
    }
}
