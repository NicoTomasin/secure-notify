// SPDX-License-Identifier: AGPL-3.0-or-later

//! The platform-neutral message model and its plain-text rendering
//! (docs/architecture/platforms.md#conversation-layer). Platform plugins render the model
//! natively; the console uses the plain-text renderer.

/// A message to one chat.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Message {
    /// Content, in order.
    pub blocks: Vec<Block>,
    /// Inline actions; rendered as buttons or as command hints.
    pub actions: Vec<Action>,
    /// A code the message hands out, so the console can offer it as `{pairing_code}`,
    /// `{invite_code}`, or `{link_code}`.
    pub generated_code: Option<GeneratedCode>,
}

/// A piece of content. Text from sources is kept as given; renderers escape it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// A title line.
    Heading(String),
    /// A paragraph.
    Text(String),
    /// A labeled value.
    Field {
        /// Label, localized.
        label: String,
        /// Value.
        value: String,
    },
    /// A list item.
    Item(String),
    /// A link; renderers show only `http` and `https` URLs.
    Link {
        /// Link text.
        label: String,
        /// Target URL.
        url: String,
    },
}

/// An inline action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    /// Button label, localized.
    pub label: String,
    /// Opaque data the platform sends back when the button is pressed.
    pub data: String,
    /// The equivalent command, for platforms without buttons.
    pub command: String,
}

/// A code handed out in a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedCode {
    /// What the code is for.
    pub kind: CodeKind,
    /// The code.
    pub value: String,
}

/// Kinds of codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeKind {
    /// The one-time pairing code.
    Pairing,
    /// An invite code.
    Invite,
    /// A link code.
    Link,
}

impl Message {
    /// A message of text paragraphs.
    pub fn text(paragraphs: impl IntoIterator<Item = String>) -> Self {
        Self {
            blocks: paragraphs.into_iter().map(Block::Text).collect(),
            ..Self::default()
        }
    }
}

/// Whether `url` may be rendered as a link (docs/architecture/security.md).
pub fn is_safe_link(url: &str) -> bool {
    let lower = url.trim_start().to_ascii_lowercase();
    lower.starts_with("https://") || lower.starts_with("http://")
}

/// Renders a message as plain text.
pub fn render_plain(message: &Message) -> String {
    let mut lines = Vec::new();
    for block in &message.blocks {
        match block {
            Block::Heading(text) | Block::Text(text) => lines.push(text.clone()),
            Block::Field { label, value } => lines.push(format!("{label}: {value}")),
            Block::Item(text) => lines.push(format!("• {text}")),
            Block::Link { label, url } if is_safe_link(url) => {
                lines.push(format!("{label}: {url}"));
            }
            Block::Link { label, .. } => lines.push(label.clone()),
        }
    }
    for action in message.actions.iter().filter(|a| !a.command.is_empty()) {
        lines.push(format!("[{}] {}", action.label, action.command));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_http_and_https_links_are_rendered() {
        // security.md: only `http`/`https` links are rendered.
        let message = Message {
            blocks: vec![
                Block::Link {
                    label: "Advisory".to_string(),
                    url: "https://example.org/a".to_string(),
                },
                Block::Link {
                    label: "Trap".to_string(),
                    url: "javascript:alert(1)".to_string(),
                },
            ],
            ..Message::default()
        };
        assert_eq!(
            render_plain(&message),
            "Advisory: https://example.org/a\nTrap"
        );
    }

    #[test]
    fn actions_show_their_command_equivalent() {
        // delivery.md#inline-actions: without buttons, the equivalent commands are listed.
        let message = Message {
            actions: vec![Action {
                label: "Mute story".to_string(),
                data: "mute:story:1".to_string(),
                command: "/mutes add story 1".to_string(),
            }],
            ..Message::default()
        };
        assert_eq!(render_plain(&message), "[Mute story] /mutes add story 1");
    }
}
