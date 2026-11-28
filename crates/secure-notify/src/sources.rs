// SPDX-License-Identifier: AGPL-3.0-or-later

//! The built-in sources, and `plugin test` and `plugin record`
//! (docs/engineering/local-development.md#developer-commands).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use secure_notify_core::ports::{HttpError, HttpRequest, HttpResponse};
use secure_notify_core::sources::{
    BoxFuture, Download, Source, SourceHttp, SourceState, normalize_items,
};
use secure_notify_declarative::FeedSource;
use secure_notify_plugin_host::{HttpSettings, SharedHttpClient};
use secure_notify_source_osv::OsvSource;

use crate::content;

/// Every built-in source: declarative sources from the embedded content.
pub(crate) fn registry() -> Result<Vec<Arc<dyn Source>>, String> {
    let mut sources: Vec<Arc<dyn Source>> = Vec::new();
    for definition in content::embedded_feeds()? {
        sources.push(Arc::new(FeedSource::new(definition)?));
    }
    sources.push(Arc::new(OsvSource::new()));
    Ok(sources)
}

/// The shared HTTP client with the default settings; bulk downloads go to `temp_dir` when given.
pub(crate) fn http_client(temp_dir: Option<PathBuf>) -> Result<Arc<SharedHttpClient>, String> {
    let mut settings = HttpSettings::new(env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_REPOSITORY"));
    settings.temp_dir = temp_dir;
    SharedHttpClient::new(settings)
}

/// Where a source's recorded fixtures live.
fn fixture_dir(id: &str) -> PathBuf {
    let native = Path::new("plugins")
        .join("sources")
        .join(id)
        .join("fixtures");
    if native.parent().is_some_and(Path::is_dir) {
        native
    } else {
        Path::new("content").join("feeds").join("fixtures").join(id)
    }
}

/// Recorded responses, `fixtures/.../index.json`: URL → body file.
type FixtureIndex = BTreeMap<String, String>;

/// Serves recorded responses; any other URL is refused, so tests never reach the network.
struct FixtureHttp {
    dir: PathBuf,
    index: FixtureIndex,
}

impl SourceHttp for FixtureHttp {
    fn get(&self, request: HttpRequest) -> BoxFuture<'_, Result<HttpResponse, HttpError>> {
        Box::pin(async move {
            let file = self
                .index
                .get(&request.url)
                .ok_or_else(|| HttpError::Refused(format!("no fixture for {}", request.url)))?;
            let body = tokio::fs::read(self.dir.join(file))
                .await
                .map_err(|e| HttpError::Transport(format!("{file}: {e}")))?;
            Ok(HttpResponse {
                status: 200,
                headers: Vec::new(),
                body,
            })
        })
    }

    fn download(
        &self,
        request: HttpRequest,
        _max_bytes: u64,
    ) -> BoxFuture<'_, Result<Download, HttpError>> {
        Box::pin(async move {
            let file = self
                .index
                .get(&request.url)
                .ok_or_else(|| HttpError::Refused(format!("no fixture for {}", request.url)))?;
            let opened = std::fs::File::open(self.dir.join(file))
                .map_err(|e| HttpError::Transport(format!("{file}: {e}")))?;
            let len = opened.metadata().map_or(0, |m| m.len());
            Ok(Download {
                status: 200,
                headers: Vec::new(),
                file: opened,
                len,
            })
        })
    }
}

/// Records every successful response of the live client.
struct RecordingHttp<'a> {
    inner: &'a dyn SourceHttp,
    recorded: Mutex<Vec<(String, Vec<u8>)>>,
}

impl SourceHttp for RecordingHttp<'_> {
    fn get(&self, request: HttpRequest) -> BoxFuture<'_, Result<HttpResponse, HttpError>> {
        Box::pin(async move {
            let url = request.url.clone();
            let response = self.inner.get(request).await?;
            if response.status == 200 {
                self.recorded
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .push((url, response.body.clone()));
            }
            Ok(response)
        })
    }

    /// Bulk downloads are passed through without recording; their fixtures are trimmed by hand.
    fn download(
        &self,
        request: HttpRequest,
        max_bytes: u64,
    ) -> BoxFuture<'_, Result<Download, HttpError>> {
        self.inner.download(request, max_bytes)
    }
}

fn find(id: &str) -> Result<Arc<dyn Source>, String> {
    registry()?
        .into_iter()
        .find(|s| s.info().id == id)
        .ok_or_else(|| format!("no source with id `{id}`"))
}

/// Writes to standard output: the events, one JSON document per line.
#[allow(clippy::print_stdout)]
fn print(line: &str) {
    println!("{line}");
}

/// `plugin test <id> [--live]`: fetches with empty state from the fixtures (or once from the
/// real source), normalizes, validates, and prints the events. Fails if any item is dropped.
pub(crate) async fn test(id: &str, live: bool) -> Result<(), String> {
    let source = find(id)?;
    let mut state = SourceState::new();
    let output = if live {
        let http = http_client(None)?.scoped(source.info().hosts.clone());
        source.fetch(&http, &mut state).await
    } else {
        let dir = fixture_dir(id);
        let index_text = std::fs::read_to_string(dir.join("index.json")).map_err(|e| {
            format!(
                "{}: {e}; record fixtures with `plugin record {id}`",
                dir.display()
            )
        })?;
        let index: FixtureIndex = serde_json::from_str(&index_text)
            .map_err(|e| format!("{}/index.json: {e}", dir.display()))?;
        source.fetch(&FixtureHttp { dir, index }, &mut state).await
    }
    .map_err(|e| format!("fetch failed: {e}"))?;
    let (mut fetched, mut valid, mut dropped) = (0, 0, 0);
    for batch in output.batches {
        let items = batch.map_err(|e| format!("fetch failed: {e}"))?;
        let (events, rejected) = normalize_items(source.as_ref(), &items);
        for event in &events {
            print(&serde_json::to_string(event).map_err(|e| e.to_string())?);
        }
        fetched += items.len();
        valid += events.len();
        dropped += rejected;
    }
    tracing::info!(
        source = id,
        live,
        fetched,
        valid,
        dropped,
        withdrawn = output.withdrawn.len(),
        "plugin test finished"
    );
    if dropped > 0 {
        return Err(format!(
            "{dropped} items failed normalization or validation"
        ));
    }
    Ok(())
}

/// `plugin record <id>`: fetches once from the real source and writes the responses as fixtures.
pub(crate) async fn record(id: &str) -> Result<(), String> {
    let source = find(id)?;
    let live = http_client(None)?.scoped(source.info().hosts.clone());
    let recording = RecordingHttp {
        inner: &live,
        recorded: Mutex::new(Vec::new()),
    };
    let mut state = SourceState::new();
    source
        .fetch(&recording, &mut state)
        .await
        .map_err(|e| format!("fetch failed: {e}"))?;
    let dir = fixture_dir(id);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut index = FixtureIndex::new();
    let recorded = recording
        .recorded
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for (n, (url, body)) in recorded.into_iter().enumerate() {
        let file = format!("response-{n}.body");
        std::fs::write(dir.join(&file), body).map_err(|e| format!("{file}: {e}"))?;
        index.insert(url, file);
    }
    let text = serde_json::to_string_pretty(&index).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("index.json"), format!("{text}\n"))
        .map_err(|e| format!("index.json: {e}"))?;
    tracing::info!(
        source = id,
        dir = %dir.display(),
        responses = index.len(),
        "fixtures recorded; trim them and write their provenance in README.md"
    );
    Ok(())
}
