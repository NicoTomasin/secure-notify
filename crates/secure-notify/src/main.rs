// SPDX-License-Identifier: AGPL-3.0-or-later

//! The secure-notify binary: command-line interface and wiring of the core to its adapters.

mod app;
mod config;
#[cfg(feature = "dev")]
mod console;
mod content;
mod logging;
mod sources;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use secure_notify_core::ports::SystemClock;
use secure_notify_storage::{Database, OpenOptions};

use crate::config::Settings;

/// Self-hosted notification bot for security and reliability events.
#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run the instance.
    Run,
    /// Apply pending database migrations (normally done at startup).
    Migrate,
    /// Write a consistent backup of the database to a new file.
    Backup {
        /// Destination file; it must not exist.
        path: PathBuf,
    },
    /// Plugin tools for contributors.
    #[command(subcommand)]
    Plugin(PluginCommand),
    /// Content tools for contributors.
    #[command(subcommand)]
    Content(ContentCommand),
    /// Development tools (built with the `dev` feature).
    #[cfg(feature = "dev")]
    #[command(subcommand)]
    Dev(DevCommand),
}

/// Plugin subcommands.
#[derive(Debug, Subcommand)]
enum PluginCommand {
    /// Run a source against its recorded fixtures (or once against the real source).
    Test {
        /// Source ID.
        id: String,
        /// Fetch once from the real source instead of the fixtures.
        #[arg(long)]
        live: bool,
    },
    /// Fetch once from the real source and record its responses as fixtures.
    Record {
        /// Source ID.
        id: String,
    },
}

/// Content subcommands.
#[derive(Debug, Subcommand)]
enum ContentCommand {
    /// Validate content files against their schemas and rules.
    Verify {
        /// Content directory.
        #[arg(long, default_value = "content")]
        dir: PathBuf,
        /// Plugins directory, whose manifests are checked too.
        #[arg(long, default_value = "plugins")]
        plugins: PathBuf,
    },
}

/// Development-only subcommands.
#[cfg(feature = "dev")]
#[derive(Debug, Subcommand)]
enum DevCommand {
    /// Run the instance with the console platform.
    Console {
        /// Run this script, print the transcript, and exit.
        #[arg(long)]
        script: Option<PathBuf>,
    },
}

/// Database file name inside the data directory.
const DATABASE_FILE: &str = "secure-notify.db";
/// Directory for automatic pre-migration backups inside the data directory.
const BACKUP_DIR: &str = "backups";

fn main() -> ExitCode {
    let cli = Cli::parse();
    // Local runs read `.env` (docs/engineering/local-development.md#environment); variables
    // already set in the environment take precedence.
    if let Err(error) = dotenvy::dotenv()
        && !error.not_found()
    {
        return fail(&format!("cannot read .env: {error}"));
    }
    let settings = match config::load(&config::ProcessSource) {
        Ok(settings) => settings,
        Err(error) => return fail(&error.to_string()),
    };
    logging::init(settings.log_level);
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => return fail(&format!("cannot start the async runtime: {error}")),
    };
    match runtime.block_on(run(cli.command, &settings)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => fail(&message),
    }
}

async fn run(command: Command, settings: &Settings) -> Result<(), String> {
    match command {
        Command::Run => {
            // At least one platform MUST be configured
            // (docs/operations/self-hosting.md#configuration).
            if settings.telegram_bot_token.is_none() {
                return Err(
                    "no platform is configured: set TELEGRAM_BOT_TOKEN or TELEGRAM_BOT_TOKEN_FILE"
                        .to_string(),
                );
            }
            let database = open_database(&settings.data_dir).await?;
            let conversation =
                app::conversation(settings, database.clone(), Arc::new(SystemClock))?;
            app::announce_pairing_code(&conversation).await?;
            app::start_pipeline(settings, &database, Arc::new(SystemClock))?;
            tracing::info!(
                version = env!("CARGO_PKG_VERSION"),
                data_dir = %settings.data_dir.display(),
                source_url = %settings.source_url,
                grace_window = %settings.instance.correlation.grace_window,
                retention_events = %settings.instance.retention.events,
                retention_dedup = %settings.instance.retention.dedup,
                "instance started"
            );
            tokio::signal::ctrl_c()
                .await
                .map_err(|e| format!("cannot wait for the shutdown signal: {e}"))?;
            tracing::info!("shutting down");
            database.close();
            Ok(())
        }
        Command::Migrate => {
            open_database(&settings.data_dir).await?;
            Ok(())
        }
        Command::Backup { path } => {
            let database = open_database(&settings.data_dir).await?;
            database
                .backup(&path)
                .await
                .map_err(|e| format!("backup failed: {e}"))?;
            tracing::info!(path = %path.display(), "backup written");
            Ok(())
        }
        Command::Plugin(PluginCommand::Test { id, live }) => sources::test(&id, live).await,
        Command::Plugin(PluginCommand::Record { id }) => sources::record(&id).await,
        Command::Content(ContentCommand::Verify { dir, plugins }) => {
            let count = content::verify(&dir)?;
            let manifests = content::verify_manifests(&plugins)?;
            tracing::info!(
                files = count,
                manifests,
                "content and plugin manifests are valid"
            );
            Ok(())
        }
        #[cfg(feature = "dev")]
        Command::Dev(DevCommand::Console { script }) => {
            let database = open_database(&settings.data_dir).await?;
            let clock = Arc::new(console::DevClock::default());
            let conversation = app::conversation(settings, database.clone(), clock.clone())?;
            app::announce_pairing_code(&conversation).await?;
            let correlator = app::start_pipeline(settings, &database, clock.clone())?;
            let ok = console::run(
                &conversation,
                &database,
                &correlator,
                &clock,
                script.as_deref(),
            )
            .await?;
            database.close();
            if ok {
                Ok(())
            } else {
                Err("console session failed: a step failed or an error was logged".to_string())
            }
        }
    }
}

async fn open_database(data_dir: &Path) -> Result<Database, String> {
    std::fs::create_dir_all(data_dir)
        .map_err(|e| format!("cannot create data directory {}: {e}", data_dir.display()))?;
    let options = OpenOptions {
        path: data_dir.join(DATABASE_FILE),
        backup_dir: data_dir.join(BACKUP_DIR),
        now: jiff::Timestamp::now(),
        readers: 4,
    };
    let (database, migration) = tokio::task::spawn_blocking(move || Database::open(&options))
        .await
        .map_err(|e| format!("cannot open the database: {e}"))?
        .map_err(|e| format!("cannot open the database: {e}"))?;
    if let Some(backup) = &migration.backup {
        tracing::info!(path = %backup.display(), "pre-migration backup written");
    }
    tracing::info!(
        from_version = migration.from_version,
        to_version = migration.to_version,
        "database schema ready"
    );
    Ok(database)
}

#[allow(clippy::print_stderr)]
fn fail(message: &str) -> ExitCode {
    tracing::error!("{message}");
    eprintln!("error: {message}");
    ExitCode::FAILURE
}
