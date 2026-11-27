// SPDX-License-Identifier: AGPL-3.0-or-later

//! Durations written as a number and a unit (`90s`, `10m`, `1h`, `7d`), used by the instance
//! config and by the console's `:advance` directive.

use jiff::SignedDuration;

/// Parses a duration such as `90s`, `10m`, `1h`, or `7d`. Days are 24 hours.
pub fn parse_duration(text: &str) -> Result<SignedDuration, String> {
    let text = text.trim();
    let split = text
        .find(|c: char| !c.is_ascii_digit())
        .ok_or_else(|| format!("`{text}` needs a unit: s, m, h, or d"))?;
    let (number, unit) = text.split_at(split);
    let number: i64 = number
        .parse()
        .map_err(|_| format!("`{text}` is not a duration such as 90s, 10m, 1h, or 7d"))?;
    let seconds_per_unit = match unit {
        "s" => 1,
        "m" => 60,
        "h" => 3_600,
        "d" => 86_400,
        _ => return Err(format!("`{text}` has an unknown unit; use s, m, h, or d")),
    };
    number
        .checked_mul(seconds_per_unit)
        .map(SignedDuration::from_secs)
        .ok_or_else(|| format!("`{text}` is too large"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documented_duration_forms_are_parsed() {
        // local-development.md#console-platform: `90s`, `10m`, `1h`, `1d`.
        let cases = [
            ("90s", 90),
            ("10m", 600),
            ("1h", 3_600),
            ("1d", 86_400),
            ("7d", 604_800),
        ];
        for (text, seconds) in cases {
            assert_eq!(
                parse_duration(text),
                Ok(SignedDuration::from_secs(seconds)),
                "{text}"
            );
        }
    }

    #[test]
    fn malformed_durations_are_rejected() {
        for text in ["", "10", "m", "10w", "1.5h", "-5m", "99999999999999999d"] {
            assert!(parse_duration(text).is_err(), "{text:?} must be rejected");
        }
    }
}
