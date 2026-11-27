// SPDX-License-Identifier: AGPL-3.0-or-later

//! Content files validate against their schemas and contribution rules
//! (docs/architecture/taxonomy.md#contribution-rules). Same validation as `content verify`.

use std::fs;
use std::path::Path;

use crate::{CheckResult, relative};

fn collect(dir: &Path, root: &Path, files: &mut Vec<(String, String)>) -> CheckResult {
    let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for path in entries.flatten().map(|e| e.path()) {
        if path.is_dir() {
            collect(&path, root, files)?;
        } else if path.extension().is_some_and(|e| e == "toml") {
            let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            files.push((relative(root, &path), text));
        }
    }
    Ok(())
}

pub(crate) fn check(root: &Path) -> CheckResult {
    let taxonomy = root.join("content").join("taxonomy");
    let mut files = Vec::new();
    if taxonomy.is_dir() {
        collect(&taxonomy, &taxonomy, &mut files)?;
    }
    secure_notify_model::load_taxonomy(&files)
        .map(|_| ())
        .map_err(|problems| problems.join("\n"))
}
