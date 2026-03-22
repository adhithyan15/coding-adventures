//! # seq — Print a Sequence of Numbers
//!
//! This module implements the business logic for the `seq` command.
//! The `seq` utility prints a sequence of numbers from FIRST to LAST,
//! separated by a configurable string (default: newline).
//!
//! ## Usage Patterns
//!
//! `seq` accepts 1, 2, or 3 numeric arguments:
//!
//! ```text
//!     Arguments     Interpretation
//!     ───────────   ─────────────────────────────
//!     seq LAST      Print 1 to LAST, step 1
//!     seq F LAST    Print F to LAST, step 1
//!     seq F INC L   Print F to L, step INC
//! ```
//!
//! ## Equal Width
//!
//! The `-w` flag pads all numbers to the same width with leading
//! zeros, based on the widest number in the sequence:
//!
//! ```text
//!     seq -w 1 10   →  01, 02, 03, ..., 10
//!     seq -w 8 12   →  08, 09, 10, 11, 12
//! ```
//!
//! ## Floating Point
//!
//! `seq` supports floating-point numbers. The number of decimal
//! places in the output matches the most precise input:
//!
//! ```text
//!     seq 0.5 2.5   →  0.5, 1.5, 2.5
//!     seq 1 0.5 3   →  1.0, 1.5, 2.0, 2.5, 3.0
//! ```

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Generate a sequence of numbers from `first` to `last` with the given
/// increment, separated by `separator`.
///
/// # Parameters
///
/// - `first`: Starting value
/// - `increment`: Step between consecutive values (can be negative)
/// - `last`: Ending value (inclusive, if reachable)
/// - `separator`: String placed between consecutive numbers
/// - `equal_width`: If true, pad with leading zeros to equal width
///
/// # Algorithm
///
/// We use a simple loop that starts at `first` and adds `increment`
/// until we exceed `last`. For positive increments, we stop when
/// the current value exceeds `last`. For negative increments, we
/// stop when the current value is less than `last`.
///
/// ```text
///     first = 1, increment = 2, last = 7
///
///     Iteration 1: value = 1  (1 <= 7 ✓)  → output "1"
///     Iteration 2: value = 3  (3 <= 7 ✓)  → output "3"
///     Iteration 3: value = 5  (5 <= 7 ✓)  → output "5"
///     Iteration 4: value = 7  (7 <= 7 ✓)  → output "7"
///     Iteration 5: value = 9  (9 <= 7 ✗)  → stop
/// ```
///
/// # Floating Point Precision
///
/// We use f64 arithmetic, which can introduce rounding errors for
/// decimal fractions. To mitigate this, we apply a small epsilon
/// tolerance when comparing against the last value.
pub fn generate_sequence(
    first: f64,
    increment: f64,
    last: f64,
    separator: &str,
    equal_width: bool,
) -> String {
    // --- Guard: zero increment would loop forever ---
    if increment == 0.0 {
        return String::new();
    }

    // --- Guard: impossible direction ---
    // If increment is positive but first > last, or
    // if increment is negative but first < last, no output.
    if (increment > 0.0 && first > last) || (increment < 0.0 && first < last) {
        return String::new();
    }

    // --- Step 1: Collect all values ---
    let mut values: Vec<f64> = Vec::new();
    let mut current = first;

    // Small epsilon for floating-point comparison tolerance.
    // This prevents off-by-one errors from floating-point rounding.
    let epsilon = increment.abs() * 1e-10;

    loop {
        if increment > 0.0 && current > last + epsilon {
            break;
        }
        if increment < 0.0 && current < last - epsilon {
            break;
        }
        values.push(current);
        current += increment;
    }

    if values.is_empty() {
        return String::new();
    }

    // --- Step 2: Determine decimal precision ---
    // We need to figure out how many decimal places to show.
    // Use the precision implied by the inputs.
    let precision = decimal_places(first)
        .max(decimal_places(increment))
        .max(decimal_places(last));

    // --- Step 3: Format each value ---
    let formatted: Vec<String> = values
        .iter()
        .map(|v| format_number(*v, precision))
        .collect();

    // --- Step 4: Apply equal width padding if requested ---
    let final_strings = if equal_width {
        let max_len = formatted.iter().map(|s| s.len()).max().unwrap_or(0);
        formatted
            .iter()
            .map(|s| {
                if s.starts_with('-') {
                    // Negative numbers: pad zeros after the minus sign
                    let rest = &s[1..];
                    let pad = max_len - s.len();
                    format!("-{}{}", "0".repeat(pad), rest)
                } else {
                    let pad = max_len - s.len();
                    format!("{}{}", "0".repeat(pad), s)
                }
            })
            .collect()
    } else {
        formatted
    };

    // --- Step 5: Join with separator and add trailing newline ---
    let mut result = final_strings.join(separator);
    result.push('\n');
    result
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Count the number of decimal places in a float.
///
/// We convert the float to a string and count digits after the decimal
/// point. If there's no decimal point, the precision is 0.
///
/// ```text
///     1.0    → 0  (we treat whole numbers as precision 0)
///     1.5    → 1
///     1.25   → 2
///     100.0  → 0
/// ```
fn decimal_places(value: f64) -> usize {
    let s = format!("{}", value);
    match s.find('.') {
        None => 0,
        Some(pos) => {
            let decimals = &s[pos + 1..];
            // If the only decimal digit is "0", treat as whole number
            if decimals == "0" {
                0
            } else {
                decimals.len()
            }
        }
    }
}

/// Format a number with a specific number of decimal places.
///
/// If precision is 0, format as an integer (no decimal point).
/// Otherwise, format with exactly `precision` decimal places.
fn format_number(value: f64, precision: usize) -> String {
    if precision == 0 {
        // Format as integer — round to nearest whole number
        format!("{}", value.round() as i64)
    } else {
        format!("{:.prec$}", value, prec = precision)
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_sequence() {
        assert_eq!(generate_sequence(1.0, 1.0, 5.0, "\n", false), "1\n2\n3\n4\n5\n");
    }

    #[test]
    fn custom_separator() {
        assert_eq!(generate_sequence(1.0, 1.0, 3.0, ", ", false), "1, 2, 3\n");
    }

    #[test]
    fn step_of_two() {
        assert_eq!(generate_sequence(1.0, 2.0, 7.0, "\n", false), "1\n3\n5\n7\n");
    }

    #[test]
    fn counting_down() {
        assert_eq!(generate_sequence(5.0, -1.0, 1.0, "\n", false), "5\n4\n3\n2\n1\n");
    }

    #[test]
    fn single_value() {
        assert_eq!(generate_sequence(5.0, 1.0, 5.0, "\n", false), "5\n");
    }

    #[test]
    fn equal_width_padding() {
        assert_eq!(
            generate_sequence(8.0, 1.0, 11.0, "\n", true),
            "08\n09\n10\n11\n"
        );
    }

    #[test]
    fn impossible_sequence() {
        assert_eq!(generate_sequence(5.0, 1.0, 1.0, "\n", false), "");
    }

    #[test]
    fn zero_increment() {
        assert_eq!(generate_sequence(1.0, 0.0, 5.0, "\n", false), "");
    }

    #[test]
    fn fractional_step() {
        assert_eq!(
            generate_sequence(0.0, 0.5, 1.5, "\n", false),
            "0.0\n0.5\n1.0\n1.5\n"
        );
    }

    #[test]
    fn decimal_places_helper() {
        assert_eq!(decimal_places(1.0), 0);
        assert_eq!(decimal_places(1.5), 1);
        assert_eq!(decimal_places(1.25), 2);
    }
}
