// SPDX-License-Identifier: AGPL-3.0-or-later

//! Checks the workspace dependency graph against the crate dependency table in
//! docs/architecture/overview.md#crate-dependencies, which is parsed so the rule lives in one
//! place.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

use crate::{CheckResult, relative};

const TABLE_DOC: &str = "docs/architecture/overview.md";
const TABLE_HEADING: &str = "## Crate dependencies";

/// What a row of the table allows.
#[derive(Debug, Default, PartialEq)]
struct Allowed {
    any: bool,
    crates: BTreeSet<String>,
    /// Extra crates allowed only for platform plugins (`plugins/platforms/`).
    platform_extra: BTreeSet<String>,
}

/// Which row of the table governs a package.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Row {
    Crate(String),
    Plugins,
}

/// Package name of a crate named in the table (`core` is `secure-notify-core`).
fn package_name(short: &str) -> String {
    match short {
        "secure-notify" | "xtask" => short.to_string(),
        other => format!("secure-notify-{other}"),
    }
}

/// Backticked words of a table cell.
fn backticked(cell: &str) -> Vec<String> {
    cell.split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

/// Parses the crate dependency table.
fn parse_table(doc: &str) -> Result<BTreeMap<Row, Allowed>, String> {
    let section = doc
        .split_once(TABLE_HEADING)
        .map(|(_, rest)| rest)
        .ok_or_else(|| format!("{TABLE_DOC}: section `{TABLE_HEADING}` not found"))?;
    let section = section.split("\n## ").next().unwrap_or(section);
    let mut table = BTreeMap::new();
    for line in section.lines().filter(|l| l.starts_with('|')).skip(2) {
        let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
        let [left, right] = cells.as_slice() else {
            return Err(format!("{TABLE_DOC}: unexpected table row `{line}`"));
        };
        let mut allowed = Allowed::default();
        let mut parts = right.split(';');
        let base = parts.next().unwrap_or_default().trim();
        match base {
            "any" => allowed.any = true,
            "none" => {}
            _ => allowed.crates = backticked(base).iter().map(|c| package_name(c)).collect(),
        }
        for extra in parts {
            let extra = extra.trim();
            if extra.starts_with("platform plugins also") {
                allowed.platform_extra =
                    backticked(extra).iter().map(|c| package_name(c)).collect();
            } else {
                return Err(format!("{TABLE_DOC}: cannot interpret `{extra}`"));
            }
        }
        if left.starts_with("Native plugins in `plugins/`") {
            table.insert(Row::Plugins, allowed);
        } else {
            let names = backticked(left);
            if names.is_empty() {
                return Err(format!("{TABLE_DOC}: no crate named in `{left}`"));
            }
            for name in names {
                table.insert(
                    Row::Crate(package_name(&name)),
                    Allowed {
                        any: allowed.any,
                        crates: allowed.crates.clone(),
                        platform_extra: allowed.platform_extra.clone(),
                    },
                );
            }
        }
    }
    Ok(table)
}

/// A workspace package and its dependencies on other workspace packages.
struct Package {
    name: String,
    /// Manifest directory relative to the root, with `/` separators.
    dir: String,
    workspace_deps: BTreeSet<String>,
}

fn workspace_packages(root: &Path) -> Result<Vec<Package>, String> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps", "--locked"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("could not run `cargo metadata`: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "`cargo metadata` failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let metadata: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("cannot parse `cargo metadata` output: {e}"))?;
    let packages = metadata["packages"].as_array().cloned().unwrap_or_default();
    let names: BTreeSet<String> = packages
        .iter()
        .filter_map(|p| p["name"].as_str().map(str::to_string))
        .collect();
    let mut result = Vec::new();
    for package in &packages {
        let name = package["name"].as_str().unwrap_or_default().to_string();
        let manifest = Path::new(package["manifest_path"].as_str().unwrap_or_default());
        let dir = relative(root, manifest.parent().unwrap_or(manifest));
        let workspace_deps = package["dependencies"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|d| d["path"].is_string())
            .filter_map(|d| d["name"].as_str())
            .filter(|n| names.contains(*n))
            .map(str::to_string)
            .collect();
        result.push(Package {
            name,
            dir,
            workspace_deps,
        });
    }
    Ok(result)
}

/// Problems in `packages` according to `table`.
fn violations(table: &BTreeMap<Row, Allowed>, packages: &[Package]) -> Vec<String> {
    let mut problems = Vec::new();
    for package in packages {
        let is_plugin = package.dir.starts_with("plugins/");
        let row = if is_plugin {
            Row::Plugins
        } else {
            Row::Crate(package.name.clone())
        };
        let Some(allowed) = table.get(&row) else {
            problems.push(format!(
                "{} ({}) is not in the crate dependency table of {TABLE_DOC}; add it there first",
                package.name, package.dir
            ));
            continue;
        };
        if allowed.any {
            continue;
        }
        let is_platform = package.dir.starts_with("plugins/platforms/");
        for dep in &package.workspace_deps {
            let ok = allowed.crates.contains(dep)
                || (is_platform && allowed.platform_extra.contains(dep));
            if !ok {
                problems.push(format!(
                    "{} must not depend on {dep} (see {TABLE_DOC}#crate-dependencies)",
                    package.name
                ));
            }
        }
    }
    problems
}

pub(crate) fn check(root: &Path) -> CheckResult {
    let doc = fs::read_to_string(root.join(TABLE_DOC)).map_err(|e| format!("{TABLE_DOC}: {e}"))?;
    let table = parse_table(&doc)?;
    let packages = workspace_packages(root)?;
    let problems = violations(&table, &packages);
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package(name: &str, dir: &str, deps: &[&str]) -> Package {
        Package {
            name: name.to_string(),
            dir: dir.to_string(),
            workspace_deps: deps.iter().map(|d| (*d).to_string()).collect(),
        }
    }

    fn documented_table() -> BTreeMap<Row, Allowed> {
        let doc = fs::read_to_string(crate::workspace_root().join(TABLE_DOC)).unwrap();
        parse_table(&doc).unwrap()
    }

    #[test]
    fn core_must_not_depend_on_an_adapter_crate() {
        // overview.md: `core` may depend on `model` and `versions` only.
        let packages = [package(
            "secure-notify-core",
            "crates/core",
            &["secure-notify-model", "secure-notify-storage"],
        )];
        let problems = violations(&documented_table(), &packages);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("must not depend on secure-notify-storage"));
    }

    #[test]
    fn only_platform_plugins_may_depend_on_conversation() {
        let packages = [
            package(
                "secure-notify-platform-x",
                "plugins/platforms/x",
                &["secure-notify-core", "secure-notify-conversation"],
            ),
            package(
                "secure-notify-source-y",
                "plugins/sources/y",
                &["secure-notify-conversation"],
            ),
        ];
        let problems = violations(&documented_table(), &packages);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].starts_with("secure-notify-source-y must not depend"));
    }

    #[test]
    fn a_crate_missing_from_the_table_is_reported() {
        let packages = [package("secure-notify-extra", "crates/extra", &[])];
        let problems = violations(&documented_table(), &packages);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("is not in the crate dependency table"));
    }

    #[test]
    fn model_may_depend_on_no_workspace_crate() {
        let packages = [package(
            "secure-notify-model",
            "crates/model",
            &["secure-notify-versions"],
        )];
        assert_eq!(violations(&documented_table(), &packages).len(), 1);
    }
}
