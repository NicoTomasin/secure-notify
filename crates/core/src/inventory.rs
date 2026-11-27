// SPDX-License-Identifier: AGPL-3.0-or-later

//! Inventory: the parser contract, the upload flow, and manual entries
//! (docs/architecture/inventory.md, docs/architecture/plugins.md#parsers).
//!
//! Uploaded files are parsed in memory and discarded; only flat component and version pairs are
//! stored, with no grouping per upload (docs/architecture/inventory.md#privacy).

use std::collections::BTreeSet;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use jiff::Timestamp;

use crate::ports::{Clock, StoreError};
use crate::subscribers::SubscriberId;
use crate::taxonomy::Taxonomy;
use crate::watches::{WatchStore, Watches};

/// Largest accepted upload.
pub const MAX_UPLOAD_BYTES: usize = 20 * 1024 * 1024;
/// Most components one upload may produce.
pub const MAX_COMPONENTS: usize = 100_000;
/// Longest a parser may run.
pub const PARSE_TIMEOUT: Duration = Duration::from_secs(10);

/// A component found in a file: a purl without version, and the exact version when the file
/// gives one.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ParsedComponent {
    /// Package URL without version.
    pub purl: String,
    /// Exact version; `None` when the file gives only a range.
    pub version: Option<String>,
}

/// What a parser extracts from a file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Parsed {
    /// Components, possibly with duplicates.
    pub components: Vec<ParsedComponent>,
    /// Technologies the file shows (`tech:npm` for an npm lock file).
    pub technologies: Vec<String>,
}

/// Why a parser could not read a file.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    /// The file is not valid for its format. The message never quotes file content.
    #[error("malformed file: {0}")]
    Malformed(String),
    /// Bun's binary lockfile, which is not parsed (docs/architecture/inventory.md).
    #[error("binary Bun lockfile")]
    BunBinaryLockfile,
}

/// The parser contract. Parsers receive bytes and the file name only; they never execute
/// content, reach the network or the file system, or keep state.
pub trait Parser: Send + Sync {
    /// Plugin ID.
    fn id(&self) -> &'static str;
    /// Whether the parser handles files with this name.
    fn accepts(&self, filename: &str) -> bool;
    /// Parses a file.
    fn parse(&self, filename: &str, bytes: &[u8]) -> Result<Parsed, ParseError>;
}

/// An inventory entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryEntry {
    /// Package URL without version.
    pub component: String,
    /// Exact version, if known.
    pub version: Option<String>,
    /// When it was last seen in an upload or added.
    pub last_seen_at: Timestamp,
}

/// Counts of a merge.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MergeCounts {
    /// New entries.
    pub added: usize,
    /// Existing entries whose `last_seen_at` was refreshed.
    pub refreshed: usize,
}

/// Persistence for inventories.
pub trait InventoryStore: Send + Sync {
    /// Adds new pairs and refreshes `last_seen_at` of existing ones; removes nothing.
    fn merge_inventory(
        &self,
        subscriber: SubscriberId,
        entries: Vec<ParsedComponent>,
        now: Timestamp,
    ) -> impl Future<Output = Result<MergeCounts, StoreError>> + Send;

    /// Entries of a subscriber, sorted by component and version.
    fn inventory(
        &self,
        subscriber: SubscriberId,
    ) -> impl Future<Output = Result<Vec<InventoryEntry>, StoreError>> + Send;

    /// Removes entries of a component: one version (`Some(Some(v))`), the entry without version
    /// (`Some(None)`), or every version (`None`). Returns how many were removed.
    fn remove_inventory(
        &self,
        subscriber: SubscriberId,
        component: &str,
        version: Option<Option<String>>,
    ) -> impl Future<Output = Result<usize, StoreError>> + Send;

    /// Removes every entry of a subscriber. Returns how many were removed.
    fn clear_inventory(
        &self,
        subscriber: SubscriberId,
    ) -> impl Future<Output = Result<usize, StoreError>> + Send;
}

/// Result of an upload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadOutcome {
    /// Entries merged.
    Merged {
        /// Counts.
        counts: MergeCounts,
        /// Technologies detected, added as derived watches.
        technologies: Vec<String>,
        /// The distinct components and versions of the file, for the initial scan.
        components: Vec<ParsedComponent>,
    },
    /// No parser handles this file name.
    Unsupported,
    /// Larger than [`MAX_UPLOAD_BYTES`].
    TooLarge,
    /// More than [`MAX_COMPONENTS`] components.
    TooManyComponents,
    /// The parser did not finish within [`PARSE_TIMEOUT`].
    TimedOut,
    /// The file could not be parsed.
    Malformed,
    /// Bun's binary lockfile.
    BunBinaryLockfile,
}

/// Result of `/inventory add`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddOutcome {
    /// Added or refreshed.
    Added,
    /// The component is not a valid purl.
    InvalidComponent(String),
}

/// Inventory operations.
pub struct Inventory<S> {
    store: S,
    parsers: Arc<[Arc<dyn Parser>]>,
    taxonomy: Arc<Taxonomy>,
    clock: Arc<dyn Clock>,
}

/// The base name of an uploaded file, without any directory part.
fn base_name(filename: &str) -> &str {
    filename.rsplit(['/', '\\']).next().unwrap_or(filename)
}

impl<S: InventoryStore + WatchStore> Inventory<S> {
    /// Creates the service.
    pub fn new(
        store: S,
        parsers: Arc<[Arc<dyn Parser>]>,
        taxonomy: Arc<Taxonomy>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            store,
            parsers,
            taxonomy,
            clock,
        }
    }

    /// File names the parsers accept, for help texts.
    pub fn supported_files(&self) -> &'static [&'static str] {
        &[
            "package.json",
            "package-lock.json",
            "npm-shrinkwrap.json",
            "pnpm-lock.yaml",
            "bun.lock",
        ]
    }

    /// The upload flow: select the parser by name, enforce limits, parse off the async executor
    /// with a timeout, merge as a union, and add derived watches. The bytes are dropped when this
    /// returns.
    pub async fn upload(
        &self,
        subscriber: SubscriberId,
        watches: &Watches<S>,
        filename: &str,
        bytes: Vec<u8>,
    ) -> Result<UploadOutcome, StoreError> {
        let name = base_name(filename).to_string();
        let Some(parser) = self.parsers.iter().find(|p| p.accepts(&name)).cloned() else {
            return Ok(UploadOutcome::Unsupported);
        };
        if bytes.len() > MAX_UPLOAD_BYTES {
            return Ok(UploadOutcome::TooLarge);
        }
        let parse = tokio::task::spawn_blocking(move || parser.parse(&name, &bytes));
        let parsed = match tokio::time::timeout(PARSE_TIMEOUT, parse).await {
            Err(_) => return Ok(UploadOutcome::TimedOut),
            Ok(Err(join_error)) => {
                tracing::error!(error = %join_error, "parser panicked");
                return Ok(UploadOutcome::Malformed);
            }
            Ok(Ok(Err(ParseError::BunBinaryLockfile))) => {
                return Ok(UploadOutcome::BunBinaryLockfile);
            }
            Ok(Ok(Err(ParseError::Malformed(_)))) => return Ok(UploadOutcome::Malformed),
            Ok(Ok(Ok(parsed))) => parsed,
        };
        let components: Vec<ParsedComponent> = parsed
            .components
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        if components.len() > MAX_COMPONENTS {
            return Ok(UploadOutcome::TooManyComponents);
        }
        let mut technologies: BTreeSet<String> = parsed.technologies.into_iter().collect();
        for component in &components {
            technologies.extend(
                self.taxonomy
                    .entities_for_purl(&component.purl)
                    .map(str::to_string),
            );
        }
        let technologies: Vec<String> = technologies
            .into_iter()
            .filter(|id| self.taxonomy.get(id).is_some())
            .collect();
        let counts = self
            .store
            .merge_inventory(subscriber, components.clone(), self.clock.now())
            .await?;
        watches.derive(subscriber, &technologies).await?;
        Ok(UploadOutcome::Merged {
            counts,
            technologies,
            components,
        })
    }

    /// `/inventory`.
    pub async fn list(&self, subscriber: SubscriberId) -> Result<Vec<InventoryEntry>, StoreError> {
        self.store.inventory(subscriber).await
    }

    /// `/inventory add <purl> [version]`.
    pub async fn add(
        &self,
        subscriber: SubscriberId,
        component: &str,
        version: Option<&str>,
    ) -> Result<AddOutcome, StoreError> {
        if let Err(reason) = secure_notify_model::validate_purl(component) {
            return Ok(AddOutcome::InvalidComponent(reason));
        }
        let entry = ParsedComponent {
            purl: component.to_string(),
            version: version.map(str::to_string),
        };
        self.store
            .merge_inventory(subscriber, vec![entry], self.clock.now())
            .await?;
        Ok(AddOutcome::Added)
    }

    /// `/inventory remove <component>[@version]`. Returns how many entries were removed.
    pub async fn remove(&self, subscriber: SubscriberId, spec: &str) -> Result<usize, StoreError> {
        // In a purl, a scope's `@` is percent-encoded, so an `@` after the last `/` separates
        // the version.
        let (component, version) = match spec.rsplit_once('@') {
            Some((component, version)) if !component.ends_with(':') && !version.contains('/') => {
                (component, Some(Some(version.to_string())))
            }
            _ => (spec, None),
        };
        self.store
            .remove_inventory(subscriber, component, version)
            .await
    }

    /// `/inventory clear`.
    pub async fn clear(&self, subscriber: SubscriberId) -> Result<usize, StoreError> {
        self.store.clear_inventory(subscriber).await
    }
}
