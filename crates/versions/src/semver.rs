// SPDX-License-Identifier: AGPL-3.0-or-later

//! Semantic Versioning 2.0.0 precedence, as used by npm and by OSV `SEMVER` ranges.

use std::cmp::Ordering;
use std::fmt;

/// A pre-release identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Identifier {
    Numeric(u64),
    Alphanumeric(String),
}

impl Ord for Identifier {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Numeric(a), Self::Numeric(b)) => a.cmp(b),
            // Numeric identifiers have lower precedence than alphanumeric ones.
            (Self::Numeric(_), Self::Alphanumeric(_)) => Ordering::Less,
            (Self::Alphanumeric(_), Self::Numeric(_)) => Ordering::Greater,
            (Self::Alphanumeric(a), Self::Alphanumeric(b)) => a.cmp(b),
        }
    }
}

impl PartialOrd for Identifier {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A semantic version. Build metadata is parsed and ignored for precedence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemVer {
    major: u64,
    minor: u64,
    patch: u64,
    pre: Vec<Identifier>,
}

fn numeric(part: &str) -> Option<u64> {
    let valid = !part.is_empty()
        && part.chars().all(|c| c.is_ascii_digit())
        && (part == "0" || !part.starts_with('0'));
    if valid { part.parse().ok() } else { None }
}

fn identifier_chars(part: &str) -> bool {
    !part.is_empty() && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

impl SemVer {
    /// Parses `MAJOR.MINOR.PATCH[-PRERELEASE][+BUILD]`, with an optional leading `v`. Returns
    /// `None` for anything else, such as ranges, tags, or URLs.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.strip_prefix('v').unwrap_or(text);
        let (rest, build) = match text.split_once('+') {
            Some((rest, build)) => (rest, Some(build)),
            None => (text, None),
        };
        if let Some(build) = build
            && !build.split('.').all(identifier_chars)
        {
            return None;
        }
        let (core, pre) = match rest.split_once('-') {
            Some((core, pre)) => (core, Some(pre)),
            None => (rest, None),
        };
        let mut numbers = core.split('.');
        let major = numeric(numbers.next()?)?;
        let minor = numeric(numbers.next()?)?;
        let patch = numeric(numbers.next()?)?;
        if numbers.next().is_some() {
            return None;
        }
        let pre = match pre {
            None => Vec::new(),
            Some(pre) => pre
                .split('.')
                .map(|part| {
                    if !identifier_chars(part) {
                        None
                    } else if part.chars().all(|c| c.is_ascii_digit()) {
                        numeric(part).map(Identifier::Numeric)
                    } else {
                        Some(Identifier::Alphanumeric(part.to_string()))
                    }
                })
                .collect::<Option<Vec<_>>>()?,
        };
        Some(Self {
            major,
            minor,
            patch,
            pre,
        })
    }

    /// The lowest version, used for OSV's `introduced: "0"`.
    pub fn zero() -> Self {
        Self {
            major: 0,
            minor: 0,
            patch: 0,
            pre: vec![Identifier::Numeric(0)],
        }
    }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(|| match (self.pre.is_empty(), other.pre.is_empty()) {
                (true, true) => Ordering::Equal,
                // A pre-release has lower precedence than the release.
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => self.pre.cmp(&other.pre),
            })
    }
}

impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for SemVer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        for (i, id) in self.pre.iter().enumerate() {
            f.write_str(if i == 0 { "-" } else { "." })?;
            match id {
                Identifier::Numeric(n) => write!(f, "{n}")?,
                Identifier::Alphanumeric(s) => f.write_str(s)?,
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precedence_follows_the_semver_specification_example() {
        // semver.org, item 11: 1.0.0-alpha < 1.0.0-alpha.1 < 1.0.0-alpha.beta < 1.0.0-beta
        // < 1.0.0-beta.2 < 1.0.0-beta.11 < 1.0.0-rc.1 < 1.0.0.
        let ordered = [
            "1.0.0-alpha",
            "1.0.0-alpha.1",
            "1.0.0-alpha.beta",
            "1.0.0-beta",
            "1.0.0-beta.2",
            "1.0.0-beta.11",
            "1.0.0-rc.1",
            "1.0.0",
            "2.0.0",
            "2.1.0",
            "2.1.1",
        ];
        for pair in ordered.windows(2) {
            let (a, b) = (
                SemVer::parse(pair[0]).unwrap(),
                SemVer::parse(pair[1]).unwrap(),
            );
            assert!(a < b, "{} < {}", pair[0], pair[1]);
        }
    }

    #[test]
    fn build_metadata_does_not_affect_precedence() {
        assert_eq!(
            SemVer::parse("1.0.0+20130313144700")
                .unwrap()
                .cmp(&SemVer::parse("1.0.0").unwrap()),
            Ordering::Equal
        );
    }

    #[test]
    fn non_versions_are_rejected() {
        // Values found in package.json and lock files that are not exact versions.
        for text in [
            "^1.2.0",
            "~1.2.0",
            ">=1.0.0",
            "1.2",
            "1.2.3.4",
            "1.2.x",
            "latest",
            "*",
            "01.2.3",
            "1.2.3-",
            "1.2.3-a..b",
            "github:user/repo#abc",
            "file:../lib",
            "1.2.3 || 2.0.0",
        ] {
            assert_eq!(SemVer::parse(text), None, "{text}");
        }
    }
}
