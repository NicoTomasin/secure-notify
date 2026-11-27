// SPDX-License-Identifier: MIT OR Apache-2.0

//! Package URL helpers shared by parsers and sources, so both sides build identical purls.

use crate::validate::check_purl;

/// The purl of an npm package name, without version: `lodash` → `pkg:npm/lodash`,
/// `@babel/core` → `pkg:npm/%40babel/core` (the purl specification percent-encodes the `@` of
/// a scope). Names are kept as published so they match advisory data.
pub fn npm_purl(name: &str) -> String {
    match name.strip_prefix('@') {
        Some(scoped) => format!("pkg:npm/%40{scoped}"),
        None => format!("pkg:npm/{name}"),
    }
}

/// Checks that `purl` is a package URL without version.
pub fn validate_purl(purl: &str) -> Result<(), String> {
    check_purl(purl)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scoped_names_encode_the_at_sign() {
        assert_eq!(npm_purl("lodash"), "pkg:npm/lodash");
        assert_eq!(npm_purl("@babel/core"), "pkg:npm/%40babel/core");
    }
}
