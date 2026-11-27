// SPDX-License-Identifier: AGPL-3.0-or-later

//! Embeds the content files in `content/` (taxonomy, declarative sources, rules) into the binary
//! (docs/architecture/content-registry.md#instance-behavior).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::{env, fs};

fn collect(dir: &Path, root: &Path, files: &mut Vec<(String, PathBuf)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for path in entries.flatten().map(|e| e.path()) {
        if path.is_dir() {
            collect(&path, root, files);
        } else if path.extension().is_some_and(|e| e == "toml") {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            files.push((relative, path));
        }
    }
}

fn main() {
    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let root = Path::new(&manifest).join("..").join("..").join("content");
    println!("cargo::rerun-if-changed={}", root.display());
    let mut files = Vec::new();
    collect(&root, &root, &mut files);
    files.sort();
    let mut code = String::from("&[\n");
    for (relative, path) in files {
        let _ = writeln!(
            code,
            "    ({relative:?}, include_str!({:?})),",
            path.display().to_string()
        );
    }
    code.push(']');
    let out = Path::new(&env::var("OUT_DIR").unwrap_or_default()).join("content.rs");
    if let Err(error) = fs::write(&out, code) {
        panic!("cannot write {}: {error}", out.display());
    }
}
