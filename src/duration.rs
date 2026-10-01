//! Parser for Freshdesk's SLA duration strings.
//!
//! Freshdesk renders durations as space-separated `<count><unit>` tokens, e.g.
//! `"0s"`, `"10m 54s"`, `"2d 18h"`, `"1w 6d 23h 44m 13s"`. Negative values
//! appear in countdown-style fields such as `cf_ttr_time` (`"-10h 9m 43s"`).
//!
//! Counts are *not* normalized: `"1d 24m 36s"` means one day plus 24 minutes,
//! not 24 hours, so tokens are summed by unit rather than rolled up.

/// Seconds per supported unit suffix.
fn unit_seconds(unit: char) -> Option<i64> {
    match unit {
        'w' => Some(604_800),
        'd' => Some(86_400),
        'h' => Some(3_600),
        'm' => Some(60),
        's' => Some(1),
        _ => None,
    }
}

/// Parses a Freshdesk duration string into seconds.
///
/// Returns `None` if the input is absent, empty, contains an unknown unit, or
/// has trailing characters that are not part of a `<count><unit>` token.
pub fn parse_duration(input: &str) -> Option<i64> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (sign, body) = match trimmed.as_bytes()[0] {
        b'-' => (-1i64, &trimmed[1..]),
        b'+' => (1i64, &trimmed[1..]),
        _ => (1i64, trimmed),
    };
    if body.is_empty() {
        return None;
    }

    let bytes = body.as_bytes();
    let mut total: i64 = 0;
    let mut idx = 0usize;
    let mut saw_token = false;

    while idx < bytes.len() {
        // Skip separators between tokens.
        if bytes[idx] == b' ' {
            idx += 1;
            continue;
        }

        // Parse the count.
        let count_start = idx;
        while idx < bytes.len() && bytes[idx].is_ascii_digit() {
            idx += 1;
        }
        if idx == count_start {
            // A digit was required here; anything else means malformed input.
            return None;
        }
        let count: i64 = body[count_start..idx].parse().ok()?;

        // Parse the unit suffix.
        if idx >= bytes.len() {
            return None;
        }
        let unit_char = body[idx..].chars().next()?;
        if !unit_char.is_ascii_alphabetic() {
            return None;
        }
        let multiplier = unit_seconds(unit_char.to_ascii_lowercase())?;
        idx += unit_char.len_utf8();

        total = total.checked_add(count.checked_mul(multiplier)?)?;
        saw_token = true;
    }

    if !saw_token {
        return None;
    }
    Some(sign * total)
}

#[cfg(test)]
mod tests {
    use super::parse_duration;

    #[test]
    fn parses_zero() {
        assert_eq!(parse_duration("0s"), Some(0));
    }

    #[test]
    fn parses_single_units() {
        assert_eq!(parse_duration("19s"), Some(19));
        assert_eq!(parse_duration("48s"), Some(48));
        assert_eq!(parse_duration("3h"), Some(10_800));
        assert_eq!(parse_duration("2h"), Some(7_200));
    }

    #[test]
    fn parses_compound_units() {
        assert_eq!(parse_duration("10m 54s"), Some(654));
        assert_eq!(parse_duration("2d 18h"), Some(2 * 86_400 + 18 * 3_600));
        assert_eq!(parse_duration("4h 33m 49s"), Some(4 * 3_600 + 33 * 60 + 49));
    }

    #[test]
    fn parses_weeks() {
        assert_eq!(
            parse_duration("1w 6d 23h 44m 13s"),
            Some(7 * 86_400 + 6 * 86_400 + 23 * 3_600 + 44 * 60 + 13)
        );
    }

    #[test]
    fn does_not_normalize_overflowing_counts() {
        // 24 minutes is 24 minutes, not 24 hours.
        assert_eq!(parse_duration("1d 24m 36s"), Some(86_400 + 24 * 60 + 36));
    }

    #[test]
    fn parses_negatives() {
        assert_eq!(
            parse_duration("-10h 9m 43s"),
            Some(-(10 * 3_600 + 9 * 60 + 43))
        );
        assert_eq!(
            parse_duration("-2d 10h 57m 28s"),
            Some(-(2 * 86_400 + 10 * 3_600 + 57 * 60 + 28))
        );
    }

    #[test]
    fn accepts_sign_and_whitespace_variants() {
        assert_eq!(parse_duration("  +5m  "), Some(300));
        assert_eq!(parse_duration("5m"), parse_duration("5m "));
    }

    #[test]
    fn rejects_malformed_input() {
        assert_eq!(parse_duration(""), None);
        assert_eq!(parse_duration("   "), None);
        assert_eq!(parse_duration("abc"), None);
        assert_eq!(parse_duration("5x"), None, "unknown unit");
        assert_eq!(parse_duration("5"), None, "missing unit");
        assert_eq!(parse_duration("m5"), None, "missing count");
        assert_eq!(parse_duration("5m junk"), None, "trailing garbage");
        assert_eq!(parse_duration("-"), None, "sign only");
    }
}
