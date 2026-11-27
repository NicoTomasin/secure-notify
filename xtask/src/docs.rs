// SPDX-License-Identifier: AGPL-3.0-or-later

//! Link check over `docs/`, `.agents/`, and the root Markdown files
//! (docs/engineering/conventions.md#continuous-integration).

use std::fs;
use std::path::{Path, PathBuf};

use crate::markdown;
use crate::{CheckResult, relative};

/// Markdown files covered by the link check.
fn markdown_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(root) {
        files.extend(
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| is_markdown(p)),
        );
    }
    for dir in ["docs", ".agents"] {
        collect(&root.join(dir), &mut files);
    }
    files.sort();
    files
}

fn is_markdown(path: &Path) -> bool {
    path.is_file() && path.extension().is_some_and(|e| e == "md")
}

fn collect(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for path in entries.flatten().map(|e| e.path()) {
        if path.is_dir() {
            collect(&path, files);
        } else if is_markdown(&path) {
            files.push(path);
        }
    }
}

pub(crate) fn check(root: &Path) -> CheckResult {
    let mut problems = Vec::new();
    for file in markdown_files(root) {
        let text = fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
        for link in markdown::links(&text) {
            if let Err(problem) = check_link(&file, &text, &link.target) {
                problems.push(format!(
                    "{}:{}: {problem}",
                    relative(root, &file),
                    link.line
                ));
            }
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
}

fn check_link(file: &Path, text: &str, target: &str) -> CheckResult {
    if target.contains("://") || target.starts_with("mailto:") {
        return Ok(());
    }
    let (path, anchor) = match target.split_once('#') {
        Some((path, anchor)) => (path, Some(anchor)),
        None => (target, None),
    };
    let (resolved, content) = if path.is_empty() {
        (file.to_path_buf(), text.to_string())
    } else {
        let resolved = file.parent().unwrap_or(file).join(path);
        if !resolved.exists() {
            return Err(format!("broken link `{target}`: {path} does not exist"));
        }
        let content = if is_markdown(&resolved) {
            fs::read_to_string(&resolved).unwrap_or_default()
        } else {
            String::new()
        };
        (resolved, content)
    };
    if let Some(anchor) = anchor {
        if !is_markdown(&resolved) {
            return Err(format!(
                "broken link `{target}`: anchors can only point into Markdown files"
            ));
        }
        if !markdown::anchors(&content).iter().any(|a| a == anchor) {
            return Err(format!(
                "broken link `{target}`: no heading with anchor #{anchor}"
            ));
        }
    }
    Ok(())
}
