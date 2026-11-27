// SPDX-License-Identifier: AGPL-3.0-or-later

//! Domain logic of secure-notify: pipeline, scheduler, correlation, matching, and delivery, with
//! the ports adapters implement (docs/architecture/overview.md).
//!
//! The core holds the domain logic and defines traits ([`ports`]) for everything it needs from the
//! outside world; other crates implement them and only the binary wires them together.

pub mod app;
pub mod correlation;
pub mod identifiers;
pub mod inventory;
pub mod ports;
pub mod retention;
pub mod secret;
pub mod severity;
pub mod sources;
pub mod subscribers;
pub mod taxonomy;
pub mod time;
pub mod watches;

pub use secret::Secret;
