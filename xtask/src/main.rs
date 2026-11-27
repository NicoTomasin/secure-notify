// SPDX-License-Identifier: AGPL-3.0-or-later

//! Development automation for secure-notify, run as `cargo xtask <task>`.
//!
//! Every pull request check runs locally through `cargo xtask ci`
//! (docs/engineering/local-development.md#developer-commands). Automation lives here, in Rust,
//! so it runs on every platform with only Cargo installed.

#![allow(clippy::print_stdout, clippy::print_stderr)]

mod cargo;
mod content;
mod deps;
mod docs;
mod headers;
mod markdown;
mod migrations;
mod schemas;
mod skills;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Result type of a check: `Err` carries the problems found, one per line.
type CheckResult = Result<(), String>;

/// A named check that `ci` runs.
struct Check {
    name: &'static str,
    about: &'static str,
    run: fn(&Path) -> CheckResult,
}

const CHECKS: &[Check] = &[
    Check {
        name: "fmt",
        about: "formatting (rustfmt)",
        run: cargo::fmt,
    },
    Check {
        name: "clippy",
        about: "lints (clippy, warnings denied)",
        run: cargo::clippy,
    },
    Check {
        name: "test",
        about: "tests (nextest and doc tests)",
        run: cargo::test,
    },
    Check {
        name: "msrv",
        about: "build with the minimum supported Rust version",
        run: cargo::msrv,
    },
    Check {
        name: "deny",
        about: "supply chain (cargo-deny)",
        run: cargo::deny,
    },
    Check {
        name: "deps",
        about: "workspace crate dependencies",
        run: deps::check,
    },
    Check {
        name: "schemas",
        about: "generated JSON Schemas are current",
        run: schemas::check,
    },
    Check {
        name: "migrations",
        about: "migration names and sequence numbers",
        run: migrations::check,
    },
    Check {
        name: "content",
        about: "content files and taxonomy rules",
        run: content::check,
    },
    Check {
        name: "headers",
        about: "SPDX license headers",
        run: headers::check,
    },
    Check {
        name: "docs",
        about: "Markdown links and anchors",
        run: docs::check,
    },
    Check {
        name: "skills",
        about: "skills index",
        run: skills::check,
    },
];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = workspace_root();
    match args.first().map(String::as_str) {
        Some("ci") => run_checks(&root, CHECKS.iter().collect()),
        Some("gen-schemas") => match schemas::generate(&root) {
            Ok(()) => ExitCode::SUCCESS,
            Err(problem) => {
                eprintln!("{problem}");
                ExitCode::FAILURE
            }
        },
        Some(name) if CHECKS.iter().any(|c| c.name == name) => {
            run_checks(&root, CHECKS.iter().filter(|c| c.name == name).collect())
        }
        _ => {
            print_usage();
            ExitCode::FAILURE
        }
    }
}

fn print_usage() {
    eprintln!("usage: cargo xtask <task>\n\ntasks:\n  ci          run every check below");
    for check in CHECKS {
        eprintln!("  {:<11} {}", check.name, check.about);
    }
    eprintln!(
        "
generators:
  gen-schemas write the JSON Schemas in schemas/ from crates/model"
    );
}

fn run_checks(root: &Path, checks: Vec<&Check>) -> ExitCode {
    let mut failed = Vec::new();
    for check in checks {
        println!("==> {} ({})", check.name, check.about);
        if let Err(problems) = (check.run)(root) {
            eprintln!("{problems}");
            eprintln!("FAILED: {}", check.name);
            failed.push(check.name);
        }
    }
    if failed.is_empty() {
        println!("all checks passed");
        ExitCode::SUCCESS
    } else {
        eprintln!("failed checks: {}", failed.join(", "));
        ExitCode::FAILURE
    }
}

/// The repository root: the parent of this crate's manifest directory.
fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .map_or(manifest_dir.clone(), Path::to_path_buf)
}

/// Path of `path` relative to `root`, with `/` separators, for messages and path rules.
fn relative(root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
