// SPDX-License-Identifier: AGPL-3.0-or-later

//! A secret value whose `Debug` and `Display` output is redacted
//! (docs/engineering/configuration.md#secrets-in-code).

use std::fmt;

/// Holds a secret from the moment it is read until it is used. Formatting never shows the value.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    /// Wraps a secret value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The secret value, for the one place that needs it (an API call, a key derivation).
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret([redacted])")
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatting_never_shows_the_value() {
        let secret = Secret::new("123456:telegram-token");
        let rendered = format!("{secret} {secret:?} {secret:#?}");
        assert!(!rendered.contains("telegram-token"), "{rendered}");
        assert_eq!(secret.expose(), "123456:telegram-token");
    }
}
