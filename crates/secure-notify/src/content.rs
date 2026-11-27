// SPDX-License-Identifier: AGPL-3.0-or-later

//! Content embedded in the binary, and `content verify` for contributors.

use std::path::Path;

use secure_notify_core::taxonomy::Taxonomy;

/// Content files embedded at build time: path relative to `content/` and text.
const EMBEDDED: &[(&str, &str)] = include!(concat!(env!("OUT_DIR"), "/content.rs"));

/// Taxonomy files of `files`, with paths relative to `content/taxonomy/`.
fn taxonomy_files<'a>(files: impl Iterator<Item = (&'a str, &'a str)>) -> Vec<(String, String)> {
    files
        .filter_map(|(path, text)| {
            path.strip_prefix("taxonomy/")
                .map(|p| (p.to_string(), text.to_string()))
        })
        .collect()
}

/// The taxonomy embedded in the binary.
pub(crate) fn embedded_taxonomy() -> Result<Taxonomy, String> {
    Taxonomy::load(&taxonomy_files(EMBEDDED.iter().copied()))
        .map_err(|problems| format!("embedded content is invalid:\n{}", problems.join("\n")))
}

fn read_tree(dir: &Path, root: &Path, files: &mut Vec<(String, String)>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for path in entries.flatten().map(|e| e.path()) {
        if path.is_dir() {
            read_tree(&path, root, files)?;
        } else if path.extension().is_some_and(|e| e == "toml") {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            let text =
                std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            files.push((relative, text));
        }
    }
    Ok(())
}

/// `content verify`: validates the content files in `dir` (default `content`).
pub(crate) fn verify(dir: &Path) -> Result<usize, String> {
    let mut files = Vec::new();
    read_tree(dir, dir, &mut files)?;
    let taxonomy = taxonomy_files(files.iter().map(|(p, t)| (p.as_str(), t.as_str())));
    Taxonomy::load(&taxonomy).map_err(|problems| problems.join("\n"))?;
    Ok(files.len())
}
