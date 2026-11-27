// SPDX-License-Identifier: AGPL-3.0-or-later

//! Minimal Markdown scanning: inline links and heading anchors, outside code.

use std::collections::HashMap;

/// An inline link `[text](target)` found in a Markdown file.
pub(crate) struct Link {
    pub(crate) line: usize,
    pub(crate) target: String,
}

/// Lines of `text` that are not inside fenced code blocks, with their 1-based numbers.
fn prose_lines(text: &str) -> Vec<(usize, &str)> {
    let mut fence: Option<&str> = None;
    let mut lines = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        let marker = ["```", "~~~"].into_iter().find(|m| trimmed.starts_with(m));
        match (fence, marker) {
            (None, Some(m)) => fence = Some(m),
            (Some(open), Some(m)) if open == m => fence = None,
            (None, None) => lines.push((index + 1, line)),
            _ => {}
        }
    }
    lines
}

/// Removes inline code spans so that links shown as code are not checked.
fn strip_code_spans(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_code = false;
    for c in line.chars() {
        if c == '`' {
            in_code = !in_code;
        } else if !in_code {
            out.push(c);
        }
    }
    out
}

/// Inline links in `text`, outside code blocks and code spans.
pub(crate) fn links(text: &str) -> Vec<Link> {
    let mut found = Vec::new();
    for (line_number, line) in prose_lines(text) {
        let line = strip_code_spans(line);
        let mut rest = line.as_str();
        while let Some(start) = rest.find("](") {
            let after = &rest[start + 2..];
            let Some(end) = after.find(')') else { break };
            let target = after[..end].split_whitespace().next().unwrap_or("");
            if !target.is_empty() {
                found.push(Link {
                    line: line_number,
                    target: target.to_string(),
                });
            }
            rest = &after[end..];
        }
    }
    found
}

/// The anchors GitHub generates for the headings of `text`, including `-N` suffixes for
/// repeated headings.
pub(crate) fn anchors(text: &str) -> Vec<String> {
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut anchors = Vec::new();
    for (_, line) in prose_lines(text) {
        let trimmed = line.trim_start();
        let level = trimmed.chars().take_while(|c| *c == '#').count();
        if level == 0 || level > 6 || !trimmed[level..].starts_with(' ') {
            continue;
        }
        let slug = slug(trimmed[level..].trim());
        let count = seen.entry(slug.clone()).or_insert(0);
        anchors.push(if *count == 0 {
            slug.clone()
        } else {
            format!("{slug}-{count}")
        });
        *count += 1;
    }
    anchors
}

/// GitHub's heading slug: links reduced to their text, lowercased, punctuation removed, spaces
/// replaced by hyphens.
pub(crate) fn slug(heading: &str) -> String {
    let mut text = String::new();
    let mut rest = heading;
    // Replace `[text](target)` by `text`.
    while let Some(open) = rest.find('[') {
        let Some(close) = rest[open..].find("](").map(|i| open + i) else {
            break;
        };
        let Some(end) = rest[close..].find(')').map(|i| close + i) else {
            break;
        };
        text.push_str(&rest[..open]);
        text.push_str(&rest[open + 1..close]);
        rest = &rest[end + 1..];
    }
    text.push_str(rest);
    text.to_lowercase()
        .chars()
        .filter_map(|c| match c {
            ' ' => Some('-'),
            c if c.is_alphanumeric() || c == '-' || c == '_' => Some(c),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_matches_github_for_headings_used_in_the_docs() {
        // Headings from the docs and the anchors GitHub renders for them.
        let cases = [
            ("Source offer and `/about`", "source-offer-and-about"),
            (
                "Attribution (additional term, AGPL section 7(b))",
                "attribution-additional-term-agpl-section-7b",
            ),
            (
                "AI-assisted and agent contributions",
                "ai-assisted-and-agent-contributions",
            ),
            ("1. Phase skills", "1-phase-skills"),
            (
                "See [the glossary](glossary.md) first",
                "see-the-glossary-first",
            ),
        ];
        for (heading, expected) in cases {
            assert_eq!(slug(heading), expected, "heading {heading:?}");
        }
    }

    #[test]
    fn repeated_headings_get_numbered_anchors() {
        let text = "# A\n## Notes\n## Notes\n";
        assert_eq!(anchors(text), ["a", "notes", "notes-1"]);
    }

    #[test]
    fn links_inside_code_are_ignored() {
        let text = "[a](x.md) `[b](y.md)`\n```\n[c](z.md)\n```\n[d](w.md#s)\n";
        let targets: Vec<_> = links(text).into_iter().map(|l| l.target).collect();
        assert_eq!(targets, ["x.md", "w.md#s"]);
    }
}
