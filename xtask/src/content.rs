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
    let mut problems = secure_notify_model::load_taxonomy(&files)
        .err()
        .unwrap_or_default();
    problems.extend(manifest_problems(&root.join("plugins"))?);
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
}

/// Problems in `plugins/<kind>/<id>/plugin.toml` files
/// (docs/architecture/plugins.md#plugin-manifest).
fn manifest_problems(plugins: &Path) -> Result<Vec<String>, String> {
    let mut problems = Vec::new();
    let Ok(kinds) = fs::read_dir(plugins) else {
        return Ok(problems);
    };
    for kind in kinds.flatten().filter(|e| e.path().is_dir()) {
        let directory = kind.file_name().to_string_lossy().into_owned();
        let entries = fs::read_dir(kind.path()).map_err(|e| format!("{directory}: {e}"))?;
        for plugin in entries.flatten().filter(|e| e.path().is_dir()) {
            let name = plugin.file_name().to_string_lossy().into_owned();
            let shown = format!("plugins/{directory}/{name}/plugin.toml");
            match fs::read_to_string(plugin.path().join("plugin.toml")) {
                Ok(text) => {
                    if let Err(found) = secure_notify_model::load_manifest(&directory, &name, &text)
                    {
                        problems.extend(found.into_iter().map(|p| format!("{shown}: {p}")));
                    }
                }
                Err(_) => problems.push(format!("{shown} is missing")),
            }
        }
    }
    Ok(problems)
}
