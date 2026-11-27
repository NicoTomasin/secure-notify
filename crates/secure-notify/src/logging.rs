// SPDX-License-Identifier: AGPL-3.0-or-later

//! Structured JSON logs on standard error (docs/engineering/conventions.md#logging). What must
//! never be logged is defined in docs/architecture/security.md#logging: callers log endpoints by
//! internal ID only and never log secrets, uploads, inventory entries, or message bodies.

use std::sync::atomic::{AtomicBool, Ordering};

use tracing::{Event, Level, Subscriber};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};

/// Set once an error has been logged; the console's script mode fails then.
static ERROR_LOGGED: AtomicBool = AtomicBool::new(false);

/// Whether an error has been logged since startup.
#[cfg_attr(not(feature = "dev"), allow(dead_code))]
pub(crate) fn error_logged() -> bool {
    ERROR_LOGGED.load(Ordering::Relaxed)
}

struct ErrorFlag;

impl<S: Subscriber> Layer<S> for ErrorFlag {
    fn on_event(&self, event: &Event<'_>, _context: Context<'_, S>) {
        if *event.metadata().level() == Level::ERROR {
            ERROR_LOGGED.store(true, Ordering::Relaxed);
        }
    }
}

/// Installs the global JSON subscriber at `level`.
pub(crate) fn init(level: Level) {
    let json = tracing_subscriber::fmt::layer()
        .json()
        .with_current_span(false)
        .with_span_list(false)
        .with_writer(std::io::stderr)
        .with_filter(LevelFilter::from_level(level));
    let subscriber = tracing_subscriber::registry().with(json).with(ErrorFlag);
    // Fails only if a subscriber is already installed, which leaves logging working.
    let _ = tracing::subscriber::set_global_default(subscriber);
}
