// SPDX-License-Identifier: AGPL-3.0-or-later

//! The secure-notify binary: command-line interface and wiring of the core to its adapters.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use secure_notify_storage::{Database, OpenOptions};

/// Self-hosted notification bot for security and reliability events.
#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Apply pending database migrations (normally done at startup).
    Migrate,
    /// Write a consistent backup of the database to a new file.
    Backup {
        /// Destination file; it must not exist.
        path: PathBuf,
    },
}

/// Database file name inside the data directory.
const DATABASE_FILE: &str = "secure-notify.db";
/// Directory for automatic pre-migration backups inside the data directory.
const BACKUP_DIR: &str = "backups";

fn main() -> ExitCode {
    let cli = Cli::parse();
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => return fail(&format!("cannot start the async runtime: {error}")),
    };
    let data_dir = PathBuf::from(std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string()));
    match runtime.block_on(run(cli.command, &data_dir)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => fail(&message),
    }
}

async fn run(command: Command, data_dir: &Path) -> Result<(), String> {
    let database = open_database(data_dir).await?;
    match command {
        Command::Migrate => Ok(()),
        Command::Backup { path } => {
            database
                .backup(&path)
                .await
                .map_err(|e| format!("backup failed: {e}"))?;
            report(&format!("backup written to {}", path.display()));
            Ok(())
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
    if migration.from_version == migration.to_version {
        report(&format!(
            "database schema is current (version {})",
            migration.to_version
        ));
    } else {
        report(&format!(
            "database schema migrated from version {} to {}",
            migration.from_version, migration.to_version
        ));
    }
    if let Some(backup) = migration.backup {
        report(&format!(
            "pre-migration backup written to {}",
            backup.display()
        ));
    }
    Ok(database)
}

/// Output of admin subcommands, for the operator running them.
#[allow(clippy::print_stdout)]
fn report(message: &str) {
    println!("{message}");
}

#[allow(clippy::print_stderr)]
fn fail(message: &str) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::FAILURE
}
