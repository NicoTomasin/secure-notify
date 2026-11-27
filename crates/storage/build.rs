// SPDX-License-Identifier: AGPL-3.0-or-later

//! Embeds the SQL migrations in `migrations/` into the binary, in sequence order. Naming and
//! numbering rules are checked by `cargo xtask migrations` (docs/engineering/migrations.md).

use std::fmt::Write as _;
use std::path::Path;
use std::{env, fs};

fn main() {
    let dir = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap_or_default()).join("migrations");
    println!("cargo::rerun-if-changed={}", dir.display());
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) => panic!("cannot read {}: {error}", dir.display()),
    };
    let mut files: Vec<(u32, String, String)> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let stem = name.strip_suffix(".sql")?;
            let (number, description) = stem.split_once('_')?;
            let number = number.parse().ok()?;
            Some((
                number,
                description.to_string(),
                entry.path().display().to_string(),
            ))
        })
        .collect();
    files.sort();
    let mut code = String::from("&[\n");
    for (number, description, path) in files {
        let _ = writeln!(
            code,
            "    Migration {{ version: {number}, description: {description:?}, sql: include_str!({path:?}) }},"
        );
    }
    code.push(']');
    let out = Path::new(&env::var("OUT_DIR").unwrap_or_default()).join("migrations.rs");
    if let Err(error) = fs::write(&out, code) {
        panic!("cannot write {}: {error}", out.display());
    }
}
