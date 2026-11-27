// SPDX-License-Identifier: AGPL-3.0-or-later

//! Structured JSON logs on standard error (docs/engineering/conventions.md#logging). What must
//! never be logged is defined in docs/architecture/security.md#logging: callers log endpoints by
//! internal ID only and never log secrets, uploads, inventory entries, or message bodies.

use tracing_subscriber::fmt;

/// Installs the global JSON subscriber at `level`.
pub(crate) fn init(level: tracing::Level) {
    let subscriber = fmt()
        .json()
        .with_max_level(level)
        .with_current_span(false)
        .with_span_list(false)
        .with_writer(std::io::stderr)
        .finish();
    // Fails only if a subscriber is already installed, which leaves logging working.
    let _ = tracing::subscriber::set_global_default(subscriber);
}
