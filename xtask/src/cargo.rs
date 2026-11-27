// SPDX-License-Identifier: AGPL-3.0-or-later

//! Checks that run Cargo and Cargo tools.

use std::path::Path;
use std::process::Command;

use crate::CheckResult;

/// Runs a command in the repository root and fails if it exits unsuccessfully.
pub(crate) fn run(root: &Path, program: &str, args: &[&str]) -> CheckResult {
    let status = Command::new(program)
        .args(args)
        .current_dir(root)
        .status()
        .map_err(|e| format!("could not run `{program}`: {e} (is it installed? see docs/engineering/local-development.md#toolchain)"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("`{program} {}` failed ({status})", args.join(" ")))
    }
}

pub(crate) fn fmt(root: &Path) -> CheckResult {
    run(root, "cargo", &["fmt", "--all", "--check"])
}

pub(crate) fn clippy(root: &Path) -> CheckResult {
    run(
        root,
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--locked",
            "--",
            "-D",
            "warnings",
        ],
    )
}

pub(crate) fn test(root: &Path) -> CheckResult {
    run(
        root,
        "cargo",
        &[
            "nextest",
            "run",
            "--workspace",
            "--all-features",
            "--locked",
            "--no-tests=pass",
        ],
    )?;
    run(
        root,
        "cargo",
        &["test", "--doc", "--workspace", "--all-features", "--locked"],
    )
}

pub(crate) fn deny(root: &Path) -> CheckResult {
    run(
        root,
        "cargo",
        &["deny", "--all-features", "--locked", "check"],
    )?;
    // The permissively licensed crates may only depend on permissively licensed crates
    // (docs/product/licensing.md#source-files-and-packages).
    run(
        root,
        "cargo",
        &[
            "deny",
            "--manifest-path",
            "crates/model/Cargo.toml",
            "--all-features",
            "--locked",
            "--config",
            "deny-permissive.toml",
            "check",
            "licenses",
        ],
    )
}

/// Builds the workspace with the minimum supported Rust version declared in the workspace
/// manifest (docs/engineering/conventions.md#rust).
pub(crate) fn msrv(root: &Path) -> CheckResult {
    let manifest =
        std::fs::read_to_string(root.join("Cargo.toml")).map_err(|e| format!("Cargo.toml: {e}"))?;
    let version = manifest
        .lines()
        .find_map(|l| l.trim().strip_prefix("rust-version = "))
        .map(|v| v.trim_matches('"').to_string())
        .ok_or("Cargo.toml: `rust-version` not found in [workspace.package]")?;
    let target_dir = root.join("target").join("msrv");
    let target_dir = target_dir.to_string_lossy();
    run(
        root,
        "rustup",
        &[
            "run",
            &version,
            "cargo",
            "check",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--locked",
            "--target-dir",
            &target_dir,
        ],
    )
    .map_err(|e| {
        format!("{e}\nthe MSRV toolchain is installed with `rustup toolchain install {version} --profile minimal`")
    })
}
