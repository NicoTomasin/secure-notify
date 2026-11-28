// SPDX-License-Identifier: AGPL-3.0-or-later

//! Wiring of the core to its adapters.

use std::sync::Arc;

use secure_notify_conversation::{About, Conversation, I18n};
use secure_notify_core::app::Core;
use secure_notify_core::correlation::Correlator;
use secure_notify_core::inventory::Parser;
use secure_notify_core::matching::Matcher;
use secure_notify_core::ports::{Clock, Random};
use secure_notify_core::sources::{
    BoxFuture, FetchOutput, RawItem, Source, SourceError, SourceHttp, SourceInfo, SourceState,
    schedule,
};
use secure_notify_parser_bun::BunParser;
use secure_notify_parser_npm::NpmParser;
use secure_notify_parser_pnpm::PnpmParser;
use secure_notify_storage::Database;

use crate::config::Settings;

/// Random bytes from the operating system's secure generator.
pub(crate) struct OsRandom;

impl Random for OsRandom {
    fn fill(&self, buf: &mut [u8]) {
        // Codes must never come from a weak source; without the OS generator the instance cannot
        // run safely, so this is treated as fatal.
        if let Err(error) = getrandom::fill(buf) {
            panic!("the operating system's random generator failed: {error}");
        }
    }
}

/// The built-in parser plugins.
pub(crate) fn parsers() -> Arc<[Arc<dyn Parser>]> {
    Arc::new([
        Arc::new(NpmParser) as Arc<dyn Parser>,
        Arc::new(PnpmParser),
        Arc::new(BunParser),
    ])
}

/// Builds the conversation layer over the database.
pub(crate) fn conversation(
    settings: &Settings,
    database: Database,
    clock: Arc<dyn Clock>,
) -> Result<Conversation<Database>, String> {
    let i18n = I18n::load().map_err(|e| format!("cannot load translations: {e}"))?;
    let taxonomy = crate::content::embedded_taxonomy()?;
    let core = Core::new(
        database,
        Arc::new(taxonomy),
        clock,
        Arc::new(OsRandom),
        parsers(),
    );
    let about = About {
        version: env!("CARGO_PKG_VERSION").to_string(),
        source_url: settings.source_url.clone(),
    };
    Ok(Conversation::new(core, i18n, about))
}

/// While no admin exists, makes sure a pairing code exists and prints it to the logs
/// (docs/operations/self-hosting.md#first-run).
pub(crate) async fn announce_pairing_code(
    conversation: &Conversation<Database>,
) -> Result<(), String> {
    let code = conversation
        .subscribers()
        .ensure_pairing_code()
        .await
        .map_err(|e| format!("cannot prepare the pairing code: {e}"))?;
    if let Some(code) = code {
        tracing::info!(
            pairing_code = %code,
            "no admin yet: send /start <pairing_code> to the bot to become the admin"
        );
    }
    Ok(())
}

/// The pipeline stages after ingestion.
pub(crate) struct Pipeline {
    /// Correlates events into stories.
    pub(crate) correlator: Arc<Correlator<Database>>,
    /// Queues stories for the subscribers they concern.
    pub(crate) matcher: Arc<Matcher<Database>>,
}

impl Pipeline {
    /// Runs every stage that has due work, until none has.
    pub(crate) async fn settle(&self) {
        match self.matcher.run_once().await {
            Ok(0) => {}
            Ok(stories) => tracing::info!(stories, "stories matched"),
            Err(error) => tracing::error!(error = %error, "matching failed"),
        }
    }
}

/// Starts the pipeline tasks: one scheduled task per enabled source, and the daily retention pass.
/// Unknown source IDs in `[sources]` stop the instance.
pub(crate) fn start_pipeline(
    settings: &Settings,
    database: &Database,
    clock: Arc<dyn Clock>,
) -> Result<Arc<Pipeline>, String> {
    let registry = crate::sources::registry()?;
    for id in settings.instance.sources.keys() {
        if !registry.iter().any(|s| &s.info().id == id) {
            return Err(format!(
                "invalid setting sources.{id}: no source with this ID"
            ));
        }
    }
    let taxonomy = Arc::new(crate::content::embedded_taxonomy()?);
    let correlator = Arc::new(Correlator::new(
        database.clone(),
        taxonomy,
        clock.clone(),
        settings.instance.correlation.grace_window,
    ));
    let temp_dir = settings.data_dir.join("tmp");
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("cannot create {}: {e}", temp_dir.display()))?;
    let http = crate::sources::http_client(Some(temp_dir))?;
    for source in registry {
        let overrides = settings.instance.sources.get(&source.info().id);
        let enabled = overrides
            .and_then(|o| o.enabled)
            .unwrap_or(source.info().default_enabled);
        if !enabled {
            tracing::info!(source = %source.info().id, "source disabled");
            continue;
        }
        let source: Arc<dyn Source> = match overrides.and_then(|o| o.interval) {
            Some(interval) => Arc::new(WithInterval::new(source, interval)),
            None => source,
        };
        let scoped: Arc<dyn SourceHttp> = Arc::new(http.scoped(source.info().hosts.clone()));
        tokio::spawn(schedule(
            source,
            scoped,
            database.clone(),
            Arc::clone(&correlator),
        ));
    }
    let (store, retention, retention_clock) = (
        database.clone(),
        settings.instance.retention.clone(),
        clock.clone(),
    );
    let matcher_clock = clock;
    tokio::spawn(async move {
        loop {
            match crate::app::retention_pass(&store, retention_clock.as_ref(), &retention).await {
                Ok(()) => {}
                Err(error) => tracing::error!(error = %error, "retention pass failed"),
            }
            tokio::time::sleep(std::time::Duration::from_hours(24)).await;
        }
    });
    let matcher = Arc::new(Matcher::new(database.clone(), matcher_clock));
    let pipeline = Arc::new(Pipeline {
        correlator,
        matcher,
    });
    let background = Arc::clone(&pipeline);
    tokio::spawn(async move {
        loop {
            background.settle().await;
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    });
    Ok(pipeline)
}

async fn retention_pass(
    store: &Database,
    clock: &dyn Clock,
    retention: &crate::config::RetentionConfig,
) -> Result<(), String> {
    let counts =
        secure_notify_core::retention::run(store, clock.now(), retention.events, retention.dedup)
            .await
            .map_err(|e| e.to_string())?;
    tracing::info!(
        payloads_expired = counts.payloads_expired,
        stories_compacted = counts.stories_compacted,
        stories_deleted = counts.stories_deleted,
        "retention pass finished"
    );
    Ok(())
}

/// A source with the operator's interval instead of its own.
struct WithInterval {
    inner: Arc<dyn Source>,
    info: SourceInfo,
}

impl WithInterval {
    fn new(inner: Arc<dyn Source>, interval: jiff::SignedDuration) -> Self {
        let mut info = inner.info().clone();
        info.interval = interval;
        Self { inner, info }
    }
}

impl Source for WithInterval {
    fn info(&self) -> &SourceInfo {
        &self.info
    }

    fn fetch<'a>(
        &'a self,
        http: &'a dyn SourceHttp,
        state: &'a mut SourceState,
    ) -> BoxFuture<'a, Result<FetchOutput, SourceError>> {
        self.inner.fetch(http, state)
    }

    fn normalize(&self, item: &RawItem) -> Result<secure_notify_model::NormalizedEvent, String> {
        self.inner.normalize(item)
    }
}
