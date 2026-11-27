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
    let taxonomy = Taxonomy::load(&taxonomy).map_err(|problems| problems.join("\n"))?;
    feed_definitions(
        files.iter().map(|(p, t)| (p.as_str(), t.as_str())),
        &taxonomy,
    )?;
    Ok(files.len())
}

/// Validates every `plugins/<kind>/<id>/plugin.toml` under `dir`. Returns how many were checked.
pub(crate) fn verify_manifests(dir: &Path) -> Result<usize, String> {
    let mut problems = Vec::new();
    let mut count = 0;
    let kinds = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for kind in kinds.flatten().filter(|e| e.path().is_dir()) {
        let directory = kind.file_name().to_string_lossy().into_owned();
        let plugins = std::fs::read_dir(kind.path()).map_err(|e| format!("{directory}: {e}"))?;
        for plugin in plugins.flatten().filter(|e| e.path().is_dir()) {
            let name = plugin.file_name().to_string_lossy().into_owned();
            let path = plugin.path().join("plugin.toml");
            let shown = format!("plugins/{directory}/{name}/plugin.toml");
            match std::fs::read_to_string(&path) {
                Ok(text) => {
                    count += 1;
                    if let Err(found) = secure_notify_model::load_manifest(&directory, &name, &text)
                    {
                        problems.extend(found.into_iter().map(|p| format!("{shown}: {p}")));
                    }
                }
                Err(_) => problems.push(format!("{shown} is missing")),
            }
        }
    }
    if problems.is_empty() {
        Ok(count)
    } else {
        Err(problems.join("\n"))
    }
}

/// Declarative sources of `files`, given as paths relative to `content/` and text.
fn feed_definitions<'a>(
    files: impl Iterator<Item = (&'a str, &'a str)>,
    taxonomy: &Taxonomy,
) -> Result<Vec<secure_notify_model::DeclarativeSource>, String> {
    let mut definitions = Vec::new();
    let mut problems = Vec::new();
    for (path, text) in files {
        let Some(stem) = path
            .strip_prefix("feeds/")
            .and_then(|p| p.strip_suffix(".toml"))
            .filter(|p| !p.contains('/'))
        else {
            continue;
        };
        match secure_notify_model::load_declarative_source(stem, text) {
            Ok(definition) => {
                for entity in &definition.defaults.entities {
                    if taxonomy.get(entity).is_none() {
                        problems.push(format!("content/{path}: unknown entity `{entity}`"));
                    }
                }
                definitions.push(definition);
            }
            Err(found) => {
                problems.extend(found.into_iter().map(|p| format!("content/{path}: {p}")))
            }
        }
    }
    if problems.is_empty() {
        Ok(definitions)
    } else {
        Err(problems.join("\n"))
    }
}

/// The declarative sources embedded in the binary.
pub(crate) fn embedded_feeds() -> Result<Vec<secure_notify_model::DeclarativeSource>, String> {
    let taxonomy = embedded_taxonomy()?;
    feed_definitions(EMBEDDED.iter().copied(), &taxonomy)
}
