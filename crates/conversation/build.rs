// SPDX-License-Identifier: AGPL-3.0-or-later

//! Embeds the Fluent files in `locales/<lang>/*.ftl` into the binary.

use std::fmt::Write as _;
use std::path::Path;
use std::{env, fs};

fn main() {
    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let dir = Path::new(&manifest).join("..").join("..").join("locales");
    println!("cargo::rerun-if-changed={}", dir.display());
    let mut languages: Vec<(String, Vec<String>)> = Vec::new();
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) => panic!("cannot read {}: {error}", dir.display()),
    };
    for entry in entries.flatten().filter(|e| e.path().is_dir()) {
        let language = entry.file_name().to_string_lossy().into_owned();
        let mut files: Vec<String> = fs::read_dir(entry.path())
            .into_iter()
            .flatten()
            .flatten()
            .map(|f| f.path())
            .filter(|p| p.extension().is_some_and(|e| e == "ftl"))
            .map(|p| p.display().to_string())
            .collect();
        files.sort();
        languages.push((language, files));
    }
    languages.sort();
    let mut code = String::from("&[\n");
    for (language, files) in languages {
        let _ = write!(code, "    ({language:?}, &[");
        for file in files {
            let _ = write!(code, "include_str!({file:?}), ");
        }
        code.push_str("]),\n");
    }
    code.push(']');
    let out = Path::new(&env::var("OUT_DIR").unwrap_or_default()).join("locales.rs");
    if let Err(error) = fs::write(&out, code) {
        panic!("cannot write {}: {error}", out.display());
    }
}
