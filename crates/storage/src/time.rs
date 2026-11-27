// SPDX-License-Identifier: AGPL-3.0-or-later

//! Stored timestamps: RFC 3339 in UTC with exactly six fractional digits, so that text order is
//! time order and SQL can compare them.

use jiff::Timestamp;

use crate::StorageError;

/// Formats a timestamp for storage.
pub(crate) fn to_text(timestamp: Timestamp) -> String {
    timestamp.strftime("%Y-%m-%dT%H:%M:%S%.6fZ").to_string()
}

/// Parses a stored timestamp.
pub(crate) fn from_text(text: &str) -> Result<Timestamp, StorageError> {
    text.parse()
        .map_err(|e| StorageError::Corrupt(format!("timestamp `{text}`: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_order_is_time_order() {
        // A whole second and a fraction after it must still sort in time order as text.
        let earlier: Timestamp = "2026-11-27T12:00:00Z".parse().unwrap();
        let later: Timestamp = "2026-11-27T12:00:00.5Z".parse().unwrap();
        assert_eq!(to_text(earlier), "2026-11-27T12:00:00.000000Z");
        assert!(to_text(earlier) < to_text(later));
        assert_eq!(from_text(&to_text(later)).unwrap(), later);
    }
}
