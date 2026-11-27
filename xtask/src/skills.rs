// SPDX-License-Identifier: AGPL-3.0-or-later

//! The skills index lists exactly the skill folders that exist, and each skill's front matter
//! names its folder (docs/engineering/workflow.md#agent-skills).

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::CheckResult;
use crate::markdown;

pub(crate) fn check(root: &Path) -> CheckResult {
    let skills_dir = root.join(".agents").join("skills");
    let index_path = skills_dir.join("README.md");
    let index =
        fs::read_to_string(&index_path).map_err(|e| format!("{}: {e}", index_path.display()))?;

    let listed: BTreeSet<String> = markdown::links(&index)
        .into_iter()
        .filter_map(|l| l.target.strip_suffix("/SKILL.md").map(str::to_string))
        .filter(|name| !name.contains('/'))
        .collect();
    let folders: BTreeSet<String> = fs::read_dir(&skills_dir)
        .map_err(|e| format!("{}: {e}", skills_dir.display()))?
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();

    let mut problems = Vec::new();
    for name in folders.difference(&listed) {
        problems.push(format!(
            ".agents/skills/{name}/ exists but is not listed in .agents/skills/README.md"
        ));
    }
    for name in listed.difference(&folders) {
        problems.push(format!(
            ".agents/skills/README.md lists `{name}`, but .agents/skills/{name}/ does not exist"
        ));
    }
    for name in folders.intersection(&listed) {
        let path = skills_dir.join(name).join("SKILL.md");
        match fs::read_to_string(&path) {
            Ok(text) => {
                if front_matter_name(&text).as_deref() != Some(name.as_str()) {
                    problems.push(format!(
                        ".agents/skills/{name}/SKILL.md: front matter `name` must be `{name}`"
                    ));
                }
            }
            Err(_) => problems.push(format!(".agents/skills/{name}/SKILL.md is missing")),
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
}

/// The `name` field of a YAML front matter block at the start of `text`.
fn front_matter_name(text: &str) -> Option<String> {
    let mut lines = text.lines();
    if lines.next()? != "---" {
        return None;
    }
    lines
        .take_while(|l| *l != "---")
        .find_map(|l| l.strip_prefix("name:"))
        .map(|v| v.trim().to_string())
}
