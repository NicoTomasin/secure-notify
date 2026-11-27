// SPDX-License-Identifier: AGPL-3.0-or-later

//! JSON Schemas in `schemas/` are generated from the canonical types in `crates/model`
//! (docs/architecture/data-model.md#canonical-schema); CI fails when they are stale.

use std::fs;
use std::path::Path;

use crate::CheckResult;

/// Rendered schema files: name and content, with a trailing newline.
fn rendered() -> Result<Vec<(&'static str, String)>, String> {
    secure_notify_model::json_schemas()
        .into_iter()
        .map(|(name, schema)| {
            serde_json::to_string_pretty(&schema)
                .map(|text| (name, format!("{text}\n")))
                .map_err(|e| format!("cannot render {name}: {e}"))
        })
        .collect()
}

/// Writes the generated schemas (`cargo xtask gen-schemas`).
pub(crate) fn generate(root: &Path) -> CheckResult {
    let dir = root.join("schemas");
    fs::create_dir_all(&dir).map_err(|e| format!("schemas/: {e}"))?;
    for (name, text) in rendered()? {
        fs::write(dir.join(name), text).map_err(|e| format!("schemas/{name}: {e}"))?;
        println!("wrote schemas/{name}");
    }
    Ok(())
}

/// Checks that the committed schemas match the generated ones.
pub(crate) fn check(root: &Path) -> CheckResult {
    let mut problems = Vec::new();
    for (name, text) in rendered()? {
        let path = root.join("schemas").join(name);
        match fs::read_to_string(&path) {
            Ok(committed) if committed.replace("\r\n", "\n") == text => {}
            Ok(_) => problems.push(format!(
                "schemas/{name} is stale; run `cargo xtask gen-schemas` and commit the result"
            )),
            Err(_) => problems.push(format!(
                "schemas/{name} is missing; run `cargo xtask gen-schemas` and commit the result"
            )),
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
}
