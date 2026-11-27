// SPDX-License-Identifier: AGPL-3.0-or-later

//! Strong identifiers found in free text and links: CVE and GHSA IDs
//! (docs/architecture/deduplication.md#correlation-signals, linked identifiers).

use std::collections::BTreeSet;

fn is_ghsa_group(group: &str) -> bool {
    group.len() == 4
        && group
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

/// CVE and GHSA IDs in `text`, normalized (`CVE-` uppercase, GHSA groups lowercase), in order of
/// first appearance and without duplicates.
pub fn extract(text: &str) -> (Vec<String>, Vec<String>) {
    let mut cves = Vec::new();
    let mut ghsas = Vec::new();
    let mut seen = BTreeSet::new();
    let upper = text.to_ascii_uppercase();
    let bytes = upper.as_bytes();
    let boundary = |i: usize| i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
    for (start, _) in upper.match_indices("CVE-") {
        if !boundary(start) {
            continue;
        }
        let rest = &upper[start + 4..];
        let year: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if year.len() != 4 || !rest[4..].starts_with('-') {
            continue;
        }
        let number: String = rest[5..].chars().take_while(char::is_ascii_digit).collect();
        let end = start + 4 + 5 + number.len();
        let after_ok = !bytes.get(end).is_some_and(u8::is_ascii_alphanumeric);
        if number.len() >= 4 && after_ok {
            let id = format!("CVE-{year}-{number}");
            if seen.insert(id.clone()) {
                cves.push(id);
            }
        }
    }
    let lower = text.to_ascii_lowercase();
    for (start, _) in lower.match_indices("ghsa-") {
        if !boundary(start) {
            continue;
        }
        let candidate = lower.get(start + 5..start + 19).unwrap_or_default();
        let groups: Vec<&str> = candidate.split('-').collect();
        let after_ok = !lower
            .as_bytes()
            .get(start + 19)
            .is_some_and(u8::is_ascii_alphanumeric);
        if groups.len() == 3 && groups.iter().all(|g| is_ghsa_group(g)) && after_ok {
            let id = format!("GHSA-{candidate}");
            if seen.insert(id.clone()) {
                ghsas.push(id);
            }
        }
    }
    (cves, ghsas)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_are_found_in_text_and_links() {
        // Hand-worked: forms seen in advisories, blog posts, and URLs.
        let text = "Fixes cve-2021-23337 and CVE-2024-12345 (see \
                    https://github.com/advisories/GHSA-35JH-R3H4-6JHM); CVE-2021-23337 again.";
        let (cves, ghsas) = extract(text);
        assert_eq!(cves, ["CVE-2021-23337", "CVE-2024-12345"]);
        assert_eq!(ghsas, ["GHSA-35jh-r3h4-6jhm"]);
    }

    #[test]
    fn lookalikes_are_not_identifiers() {
        for text in [
            "XCVE-2021-23337",
            "CVE-21-1234",
            "CVE-2021-123",
            "CVE-2021-12345X",
            "GHSA-35jh-r3h4",
            "GHSA-35jh-r3h4-6jhmz",
        ] {
            let (cves, ghsas) = extract(text);
            assert!(
                cves.is_empty() && ghsas.is_empty(),
                "{text}: {cves:?} {ghsas:?}"
            );
        }
    }
}
