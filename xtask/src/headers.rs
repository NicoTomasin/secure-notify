// SPDX-License-Identifier: AGPL-3.0-or-later

//! Every source file starts with the SPDX header of its path's license
//! (docs/product/licensing.md#source-files-and-packages).

use std::fs;
use std::path::Path;
use std::process::Command;

use crate::CheckResult;

/// Paths licensed MIT OR Apache-2.0 (docs/product/licensing.md#permissively-licensed-parts).
const PERMISSIVE_PATHS: &[&str] = &["sdk/", "schemas/", "crates/model/"];

/// Comment prefix per file extension with an SPDX header.
fn comment_prefix(path: &str) -> Option<&'static str> {
    let extension = path.rsplit_once('.').map(|(_, e)| e)?;
    match extension {
        "rs" | "wit" => Some("//"),
        "toml" | "ftl" | "sh" => Some("#"),
        "sql" => Some("--"),
        _ => None,
    }
}

/// The license identifier a file at `path` must declare.
fn expected_license(path: &str) -> &'static str {
    if PERMISSIVE_PATHS.iter().any(|p| path.starts_with(p)) {
        "MIT OR Apache-2.0"
    } else {
        "AGPL-3.0-or-later"
    }
}

/// The problem with the header of `path`, if any.
fn header_problem(path: &str, text: &str) -> Option<String> {
    let prefix = comment_prefix(path)?;
    let expected = format!(
        "{prefix} SPDX-License-Identifier: {}",
        expected_license(path)
    );
    // A shebang line may precede the header.
    let first = text
        .lines()
        .find(|l| !l.starts_with("#!"))
        .unwrap_or_default();
    if first.trim_end() == expected {
        None
    } else {
        Some(format!("{path}: first line must be `{expected}`"))
    }
}

/// Files tracked or about to be tracked by git (untracked but not ignored).
fn repository_files(root: &Path) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .args([
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ])
        .current_dir(root)
        .output()
        .map_err(|e| format!("could not run `git ls-files`: {e}"))?;
    if !output.status.success() {
        return Err("`git ls-files` failed".to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect())
}

pub(crate) fn check(root: &Path) -> CheckResult {
    let mut problems = Vec::new();
    for path in repository_files(root)? {
        if comment_prefix(&path).is_none() || !root.join(&path).is_file() {
            continue;
        }
        let text = fs::read_to_string(root.join(&path)).map_err(|e| format!("{path}: {e}"))?;
        problems.extend(header_problem(&path, &text));
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permissive_paths_require_the_permissive_identifier() {
        let agpl = "// SPDX-License-Identifier: AGPL-3.0-or-later\n";
        assert!(header_problem("crates/model/src/lib.rs", agpl).is_some());
        let permissive = "// SPDX-License-Identifier: MIT OR Apache-2.0\n";
        assert_eq!(header_problem("crates/model/src/lib.rs", permissive), None);
    }

    #[test]
    fn files_without_a_header_are_reported() {
        let problem = header_problem("crates/core/src/lib.rs", "//! Docs\n").unwrap();
        assert!(problem.contains("AGPL-3.0-or-later"), "{problem}");
    }

    #[test]
    fn toml_and_fluent_files_use_hash_comments() {
        let header = "# SPDX-License-Identifier: AGPL-3.0-or-later\nx = 1\n";
        assert_eq!(header_problem("content/feeds/a.toml", header), None);
        assert_eq!(header_problem("locales/en/main.ftl", header), None);
        assert!(header_problem("locales/en/main.ftl", "hello = Hello\n").is_some());
    }
}
