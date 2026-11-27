// SPDX-License-Identifier: AGPL-3.0-or-later

//! Migration files are named `NNNN_<description>.sql`, numbered from 0001 with no gaps and no
//! duplicates (docs/engineering/migrations.md#writing-a-migration).

use std::fs;
use std::path::Path;

use crate::CheckResult;

const DIR: &str = "crates/storage/migrations";

/// Problems with a set of migration file names.
fn problems(names: &[String]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut numbers = Vec::new();
    for name in names {
        let parsed = name.strip_suffix(".sql").and_then(|stem| {
            let (number, description) = stem.split_once('_')?;
            let valid_number = number.len() == 4 && number.chars().all(|c| c.is_ascii_digit());
            let valid_description = !description.is_empty()
                && description
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
            (valid_number && valid_description)
                .then(|| number.parse::<u32>().ok())
                .flatten()
        });
        match parsed {
            Some(number) => numbers.push((number, name)),
            None => problems.push(format!(
                "{DIR}/{name}: must be named NNNN_<snake_case_description>.sql"
            )),
        }
    }
    numbers.sort();
    for pair in numbers.windows(2) {
        if pair[0].0 == pair[1].0 {
            problems.push(format!(
                "{DIR}: {} and {} share sequence number {:04}; renumber the newer one",
                pair[0].1, pair[1].1, pair[0].0
            ));
        }
    }
    let mut expected = 1;
    for (number, name) in &numbers {
        if *number > expected {
            problems.push(format!(
                "{DIR}/{name}: expected sequence number {expected:04}; numbers must have no gaps"
            ));
        }
        expected = number + 1;
    }
    problems
}

pub(crate) fn check(root: &Path) -> CheckResult {
    let dir = root.join(DIR);
    let names: Vec<String> = fs::read_dir(&dir)
        .map_err(|e| format!("{DIR}: {e}"))?
        .flatten()
        .filter(|e| e.path().is_file())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    let problems = problems(&names);
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|n| (*n).to_string()).collect()
    }

    #[test]
    fn contiguous_well_named_migrations_pass() {
        assert_eq!(
            problems(&names(&["0001_initial.sql", "0002_add_x.sql"])),
            Vec::<String>::new()
        );
    }

    #[test]
    fn duplicate_sequence_numbers_are_rejected() {
        let found = problems(&names(&["0001_initial.sql", "0002_a.sql", "0002_b.sql"]));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("share sequence number 0002"));
    }

    #[test]
    fn gaps_are_rejected() {
        let found = problems(&names(&["0001_initial.sql", "0003_c.sql"]));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("expected sequence number 0002"));
    }

    #[test]
    fn badly_named_files_are_rejected() {
        for bad in ["2_add_x.sql", "0002_Add-X.sql", "0002_.sql", "notes.txt"] {
            let found = problems(&names(&["0001_initial.sql", bad]));
            assert_eq!(found.len(), 1, "{bad}: {found:?}");
            assert!(found[0].contains("must be named"), "{bad}: {found:?}");
        }
    }
}
