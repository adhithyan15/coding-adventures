//! # sleep — Delay for a Specified Time
//!
//! This module implements the business logic for the `sleep` command.
//! The `sleep` utility suspends execution for at least the specified
//! number of seconds.
//!
//! ## Duration Syntax
//!
//! Each duration argument is a number optionally followed by a suffix:
//!
//! ```text
//!     Suffix  Meaning     Example
//!     ──────  ──────────  ──────────────────
//!     s       seconds     sleep 5s   (5 sec)
//!     m       minutes     sleep 2m   (120 sec)
//!     h       hours       sleep 1h   (3600 sec)
//!     d       days        sleep 1d   (86400 sec)
//!     (none)  seconds     sleep 5    (5 sec)
//! ```
//!
//! ## Multiple Arguments
//!
//! When given multiple arguments, `sleep` sums them:
//!
//! ```text
//!     sleep 1m 30s   →  sleeps for 90 seconds
//!     sleep 1h 30m   →  sleeps for 5400 seconds
//! ```
//!
//! ## Floating Point
//!
//! Fractional values are supported:
//!
//! ```text
//!     sleep 0.5      →  sleeps for 500 milliseconds
//!     sleep 1.5m     →  sleeps for 90 seconds
//! ```
//!
//! ## Implementation
//!
//! The `parse_duration` function converts a string like "1.5m" into
//! a `std::time::Duration`. The `parse_durations` function handles
//! multiple arguments by parsing each and summing them.

use std::time::Duration;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a single duration string into a `Duration`.
///
/// The input format is a number optionally followed by a suffix
/// (s, m, h, d). If no suffix is given, seconds are assumed.
///
/// # Errors
///
/// Returns an error if the string cannot be parsed as a valid
/// duration (e.g., "abc", "-5", "3x").
///
/// # Examples
///
/// ```text
///     parse_duration("5")    => Ok(Duration::from_secs(5))
///     parse_duration("5s")   => Ok(Duration::from_secs(5))
///     parse_duration("2m")   => Ok(Duration::from_secs(120))
///     parse_duration("1.5h") => Ok(Duration::from_secs(5400))
///     parse_duration("1d")   => Ok(Duration::from_secs(86400))
///     parse_duration("abc")  => Err("...")
/// ```
pub fn parse_duration(input: &str) -> Result<Duration, String> {
    // --- Step 1: Separate the number from the suffix ---
    // We scan from the end of the string to find where the number
    // ends and the suffix begins.
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return Err("sleep: missing operand".to_string());
    }

    // --- Determine suffix and numeric part ---
    // The suffix is the last character if it's alphabetic.
    let (number_str, multiplier) = match trimmed.chars().last() {
        Some('s') => (&trimmed[..trimmed.len() - 1], 1.0_f64),
        Some('m') => (&trimmed[..trimmed.len() - 1], 60.0_f64),
        Some('h') => (&trimmed[..trimmed.len() - 1], 3600.0_f64),
        Some('d') => (&trimmed[..trimmed.len() - 1], 86400.0_f64),
        Some(c) if c.is_ascii_digit() || c == '.' => (trimmed, 1.0_f64),
        Some(c) => return Err(format!("sleep: invalid suffix '{}' in '{}'", c, input)),
        None => return Err("sleep: missing operand".to_string()),
    };

    // --- Step 2: Parse the numeric part ---
    let number: f64 = number_str
        .parse()
        .map_err(|_| format!("sleep: invalid time interval '{}'", input))?;

    // --- Step 3: Validate ---
    if number < 0.0 {
        return Err(format!("sleep: invalid time interval '{}'", input));
    }

    if number.is_nan() || number.is_infinite() {
        return Err(format!("sleep: invalid time interval '{}'", input));
    }

    // --- Step 4: Calculate total seconds ---
    let total_secs = number * multiplier;

    Ok(Duration::from_secs_f64(total_secs))
}

/// Parse multiple duration strings and sum them.
///
/// This handles the case where `sleep` is given multiple arguments:
/// `sleep 1m 30s` parses each individually and returns the sum.
///
/// # Parameters
///
/// - `args`: Slice of duration strings to parse and sum
///
/// # Returns
///
/// The total `Duration`, or an error if any argument is invalid.
///
/// # Example
///
/// ```text
///     parse_durations(&["1m", "30s"]) => Ok(Duration::from_secs(90))
/// ```
pub fn parse_durations(args: &[String]) -> Result<Duration, String> {
    if args.is_empty() {
        return Err("sleep: missing operand".to_string());
    }

    let mut total = Duration::ZERO;

    for arg in args {
        let d = parse_duration(arg)?;
        total += d;
    }

    Ok(total)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Basic parsing ---

    #[test]
    fn parse_plain_seconds() {
        let d = parse_duration("5").unwrap();
        assert_eq!(d, Duration::from_secs(5));
    }

    #[test]
    fn parse_seconds_suffix() {
        let d = parse_duration("5s").unwrap();
        assert_eq!(d, Duration::from_secs(5));
    }

    #[test]
    fn parse_minutes() {
        let d = parse_duration("2m").unwrap();
        assert_eq!(d, Duration::from_secs(120));
    }

    #[test]
    fn parse_hours() {
        let d = parse_duration("1h").unwrap();
        assert_eq!(d, Duration::from_secs(3600));
    }

    #[test]
    fn parse_days() {
        let d = parse_duration("1d").unwrap();
        assert_eq!(d, Duration::from_secs(86400));
    }

    // --- Floating point ---

    #[test]
    fn parse_fractional_seconds() {
        let d = parse_duration("0.5").unwrap();
        assert_eq!(d, Duration::from_millis(500));
    }

    #[test]
    fn parse_fractional_minutes() {
        let d = parse_duration("1.5m").unwrap();
        assert_eq!(d, Duration::from_secs(90));
    }

    #[test]
    fn parse_fractional_hours() {
        let d = parse_duration("0.5h").unwrap();
        assert_eq!(d, Duration::from_secs(1800));
    }

    // --- Zero ---

    #[test]
    fn parse_zero() {
        let d = parse_duration("0").unwrap();
        assert_eq!(d, Duration::ZERO);
    }

    #[test]
    fn parse_zero_with_suffix() {
        let d = parse_duration("0s").unwrap();
        assert_eq!(d, Duration::ZERO);
    }

    // --- Multiple durations ---

    #[test]
    fn sum_multiple_durations() {
        let args: Vec<String> = vec!["1m".into(), "30s".into()];
        let d = parse_durations(&args).unwrap();
        assert_eq!(d, Duration::from_secs(90));
    }

    #[test]
    fn sum_mixed_units() {
        let args: Vec<String> = vec!["1h".into(), "30m".into(), "15s".into()];
        let d = parse_durations(&args).unwrap();
        assert_eq!(d, Duration::from_secs(3600 + 1800 + 15));
    }

    // --- Error cases ---

    #[test]
    fn error_on_empty() {
        let args: Vec<String> = vec![];
        assert!(parse_durations(&args).is_err());
    }

    #[test]
    fn error_on_invalid_string() {
        assert!(parse_duration("abc").is_err());
    }

    #[test]
    fn error_on_invalid_suffix() {
        assert!(parse_duration("5x").is_err());
    }

    #[test]
    fn error_on_empty_string() {
        assert!(parse_duration("").is_err());
    }

    #[test]
    fn error_on_negative() {
        assert!(parse_duration("-5").is_err());
    }

    // --- Edge cases ---

    #[test]
    fn large_value() {
        let d = parse_duration("365d").unwrap();
        assert_eq!(d, Duration::from_secs(365 * 86400));
    }

    #[test]
    fn very_small_fraction() {
        let d = parse_duration("0.001").unwrap();
        assert_eq!(d, Duration::from_millis(1));
    }
}
