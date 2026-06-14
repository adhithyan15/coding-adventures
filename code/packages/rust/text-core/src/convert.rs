//! TEXT / VALUE / NUMBERVALUE / FIXED / DOLLAR — number ↔ string conversion.
//!
//! These functions cover the Excel surface of "number formatting" and
//! "string-to-number parsing". Full Excel format strings (with `[Red]`,
//! conditional sections, date/time, etc.) are out of scope; that work lives
//! in the future `cell-format` crate. Here we implement only the **basic
//! format codes** listed in the spec:
//!
//! | code         | meaning                                  | example         |
//! |--------------|------------------------------------------|-----------------|
//! | `0`          | integer, at least 1 digit                | `5` → `"5"`     |
//! | `0.00`       | exactly 2 fractional digits              | `5` → `"5.00"`  |
//! | `#,##0`      | integer with comma thousands             | `12345` → `"12,345"` |
//! | `#,##0.00`   | as above with 2 fractional digits        | `12345` → `"12,345.00"` |
//! | `0%`         | integer percentage                       | `0.5` → `"50%"` |
//! | `0.00E+00`   | scientific, 2 fractional + 2-digit exp   | `12345` → `"1.23E+04"` |
//!
//! Anything else is `FormatError`.

use crate::{iter_character, TextError};
use r_vector::Character;

/// `TEXT(value, format_code)` for the six basic codes above.
///
/// Numbers are rounded half-away-from-zero (Excel's default behaviour for
/// display rounding).
pub fn text(value: f64, format_code: &str) -> Result<String, TextError> {
    match format_code {
        "0" => Ok(format_integer(value, false)),
        "0.00" => Ok(format_fixed(value, 2, false)),
        "#,##0" => Ok(format_integer(value, true)),
        "#,##0.00" => Ok(format_fixed(value, 2, true)),
        "0%" => Ok(format_percent(value)),
        "0.00E+00" => Ok(format_scientific(value)),
        other => Err(TextError::FormatError {
            function: "TEXT",
            format: other.to_string(),
        }),
    }
}

/// `VALUE(text)` — best-effort parse to `f64`.
///
/// - Surrounding whitespace is ignored.
/// - A trailing `%` divides by 100.
/// - A leading `+` is accepted.
/// - Internal commas are ignored (Excel: `VALUE("1,234.56") == 1234.56`).
/// - Scientific notation is accepted via `f64::from_str`.
/// - Empty input or unparseable input → `ParseError`.
pub fn value(text: &str) -> Result<f64, TextError> {
    let raw = text.trim();
    if raw.is_empty() {
        return Err(TextError::ParseError {
            function: "VALUE",
            input: text.to_string(),
        });
    }
    let (body, percent) = if let Some(stripped) = raw.strip_suffix('%') {
        (stripped.trim_end(), true)
    } else {
        (raw, false)
    };

    // Strip commas (thousands separators in en-US locale).
    let cleaned: String = body.chars().filter(|c| *c != ',').collect();
    match cleaned.parse::<f64>() {
        Ok(n) => Ok(if percent { n / 100.0 } else { n }),
        Err(_) => Err(TextError::ParseError {
            function: "VALUE",
            input: text.to_string(),
        }),
    }
}

/// `NUMBERVALUE(text, [decimal], [group])` — locale-aware variant of `VALUE`.
///
/// - `decimal` is the decimal separator char (defaults to `'.'`).
/// - `group` is the thousands separator char (defaults to `','`).
/// - A trailing `%` divides by 100 (and may be repeated — `NUMBERVALUE("5%%")
///   == 0.0005`, matching Excel).
pub fn numbervalue(
    text: &str,
    decimal: Option<char>,
    group: Option<char>,
) -> Result<f64, TextError> {
    let dec = decimal.unwrap_or('.');
    let grp = group.unwrap_or(',');
    if dec == grp {
        return Err(TextError::BadParameter {
            name: "decimal/group",
            value: format!("{dec}={grp}"),
        });
    }
    let raw = text.trim();
    if raw.is_empty() {
        return Err(TextError::ParseError {
            function: "NUMBERVALUE",
            input: text.to_string(),
        });
    }

    // Strip and count trailing % chars.
    let mut percent_count = 0;
    let mut body: String = raw.to_string();
    while let Some(stripped) = body.strip_suffix('%') {
        percent_count += 1;
        body = stripped.trim_end().to_string();
    }

    // Replace group separator with nothing, decimal with '.'.
    let mut buf = String::with_capacity(body.len());
    for c in body.chars() {
        if c == grp {
            continue;
        }
        if c == dec {
            buf.push('.');
        } else {
            buf.push(c);
        }
    }
    let parsed: f64 = buf.parse().map_err(|_| TextError::ParseError {
        function: "NUMBERVALUE",
        input: text.to_string(),
    })?;
    Ok(parsed / 100f64.powi(percent_count as i32))
}

/// `FIXED(number, decimals, [no_commas])` — format `number` with exactly
/// `decimals` fractional digits.
///
/// - `decimals` can be negative; that rounds to the left of the decimal point
///   (`FIXED(12345, -2) == "12,300"`).
/// - `no_commas == true` suppresses the thousands separator.
pub fn fixed(number: f64, decimals: i64, no_commas: bool) -> Result<String, TextError> {
    if decimals >= 0 {
        Ok(format_fixed(number, decimals as usize, !no_commas))
    } else {
        // Round to the left of the decimal.
        let factor = 10f64.powi(-(decimals as i32));
        let rounded = (number / factor).round() * factor;
        Ok(format_integer(rounded, !no_commas))
    }
}

/// `DOLLAR(number, [decimals])` — currency string with `$` prefix and
/// configurable fractional digits. Negative numbers use the parenthesised
/// `($1,234.56)` accounting style, matching Excel's default en-US locale.
pub fn dollar(number: f64, decimals: i64) -> Result<String, TextError> {
    let body = fixed(number.abs(), decimals, false)?;
    if number < 0.0 {
        Ok(format!("(${body})"))
    } else {
        Ok(format!("${body}"))
    }
}

// ----------------------------------------------------------------------------
// Vector variants.
// ----------------------------------------------------------------------------

/// Vector `VALUE`. NA in / NA out; parse errors collapse to NA.
pub fn value_vec(x: &Character) -> Vec<Option<f64>> {
    iter_character(x)
        .map(|cell| cell.and_then(|s| value(s).ok()))
        .collect()
}

// ----------------------------------------------------------------------------
// Internal formatting helpers.
// ----------------------------------------------------------------------------

/// Insert thousands separators into the integer portion of an already-printed
/// number. Operates on the part **before** the decimal point.
fn add_thousands(int_part: &str) -> String {
    let (sign, digits) = if let Some(rest) = int_part.strip_prefix('-') {
        ("-", rest)
    } else {
        ("", int_part)
    };
    let bytes = digits.as_bytes();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + sign.len());
    out.push_str(sign);
    let n = bytes.len();
    for (i, b) in bytes.iter().enumerate() {
        let remaining = n - i;
        if i > 0 && remaining % 3 == 0 {
            out.push(',');
        }
        out.push(*b as char);
    }
    out
}

/// Format `value` rounded to integer. `commas` controls thousands separators.
fn format_integer(value: f64, commas: bool) -> String {
    // Half-away-from-zero rounding.
    let rounded = value.round();
    let s = format!("{rounded:.0}");
    if commas {
        add_thousands(&s)
    } else {
        s
    }
}

/// Format `value` with exactly `decimals` fractional digits, rounded
/// half-away-from-zero to match Excel's display rounding. `commas` controls
/// thousands separators in the integer part.
///
/// We can't rely on Rust's `{:.N}` format because it uses round-half-to-even
/// (banker's rounding), so e.g. `format!("{:.0}", 1234.5)` gives `"1234"`.
/// Excel users expect `"1235"`. We therefore pre-round at the requested
/// scale, then format with `{:.N}` to pad zeros.
fn format_fixed(value: f64, decimals: usize, commas: bool) -> String {
    let factor = 10f64.powi(decimals as i32);
    // `(x * factor).round()` is half-away-from-zero in Rust (`f64::round`).
    let rounded = (value * factor).round() / factor;
    let s = format!("{rounded:.*}", decimals);
    if !commas {
        return s;
    }
    if let Some(dot) = s.find('.') {
        let (head, tail) = s.split_at(dot);
        format!("{}{}", add_thousands(head), tail)
    } else {
        add_thousands(&s)
    }
}

/// Format as integer percent (e.g. 0.5 -> "50%").
fn format_percent(value: f64) -> String {
    let scaled = value * 100.0;
    format!("{}%", format_integer(scaled, false))
}

/// Format as `0.00E+00` (one leading digit, two fractional, signed
/// two-digit exponent).
fn format_scientific(value: f64) -> String {
    if value == 0.0 {
        return "0.00E+00".to_string();
    }
    let abs = value.abs();
    let exp = abs.log10().floor() as i32;
    let mantissa = value / 10f64.powi(exp);
    // Re-round in case mantissa landed at exactly 10.0 after fractional
    // rounding (e.g. 9.999 -> "10.00"). If so, bump exponent.
    let mantissa_str = format!("{mantissa:.2}");
    if mantissa_str.starts_with("10") || mantissa_str.starts_with("-10") {
        let bumped_exp = exp + 1;
        let bumped_mantissa = value / 10f64.powi(bumped_exp);
        return format!(
            "{:.2}E{}{:02}",
            bumped_mantissa,
            if bumped_exp >= 0 { '+' } else { '-' },
            bumped_exp.abs()
        );
    }
    format!(
        "{}E{}{:02}",
        mantissa_str,
        if exp >= 0 { '+' } else { '-' },
        exp.abs()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_basic_codes() {
        assert_eq!(text(5.0, "0").unwrap(), "5");
        assert_eq!(text(5.4, "0").unwrap(), "5");
        assert_eq!(text(5.6, "0").unwrap(), "6");
        assert_eq!(text(-3.2, "0").unwrap(), "-3");

        assert_eq!(text(5.0, "0.00").unwrap(), "5.00");
        assert_eq!(text(1234.567, "0.00").unwrap(), "1234.57");

        assert_eq!(text(12345.0, "#,##0").unwrap(), "12,345");
        assert_eq!(text(12345.678, "#,##0.00").unwrap(), "12,345.68");
        assert_eq!(text(-12345.0, "#,##0").unwrap(), "-12,345");
        assert_eq!(text(0.0, "#,##0").unwrap(), "0");
    }

    #[test]
    fn text_percent() {
        assert_eq!(text(0.5, "0%").unwrap(), "50%");
        assert_eq!(text(0.0, "0%").unwrap(), "0%");
        assert_eq!(text(1.234, "0%").unwrap(), "123%");
    }

    #[test]
    fn text_scientific() {
        assert_eq!(text(12345.0, "0.00E+00").unwrap(), "1.23E+04");
        assert_eq!(text(0.0001234, "0.00E+00").unwrap(), "1.23E-04");
        assert_eq!(text(0.0, "0.00E+00").unwrap(), "0.00E+00");
        assert_eq!(text(-12345.0, "0.00E+00").unwrap(), "-1.23E+04");
    }

    #[test]
    fn text_bad_format() {
        let e = text(1.0, "@@bad@@").unwrap_err();
        assert!(matches!(e, TextError::FormatError { function: "TEXT", .. }));
    }

    #[test]
    fn value_basic() {
        assert_eq!(value("123").unwrap(), 123.0);
        assert_eq!(value("3.14").unwrap(), 3.14);
        assert_eq!(value("-7.5").unwrap(), -7.5);
        assert_eq!(value("+1.5").unwrap(), 1.5);
    }

    #[test]
    fn value_whitespace_and_commas() {
        assert_eq!(value("   42  ").unwrap(), 42.0);
        assert_eq!(value("1,234.56").unwrap(), 1234.56);
    }

    #[test]
    fn value_percent() {
        assert_eq!(value("50%").unwrap(), 0.5);
        assert_eq!(value("100%").unwrap(), 1.0);
    }

    #[test]
    fn value_scientific() {
        assert_eq!(value("1.23e4").unwrap(), 12300.0);
        assert_eq!(value("1.5E-2").unwrap(), 0.015);
    }

    #[test]
    fn value_failures() {
        assert!(value("").is_err());
        assert!(value("abc").is_err());
        assert!(value("12abc").is_err());
    }

    #[test]
    fn numbervalue_default_is_value() {
        assert_eq!(numbervalue("1,234.56", None, None).unwrap(), 1234.56);
        assert_eq!(numbervalue("50%", None, None).unwrap(), 0.5);
        // Multiple % stacks.
        assert!((numbervalue("5%%", None, None).unwrap() - 0.0005).abs() < 1e-12);
    }

    #[test]
    fn numbervalue_eu_locale() {
        // German style: '.' for thousands, ',' for decimal.
        assert_eq!(
            numbervalue("1.234,56", Some(','), Some('.')).unwrap(),
            1234.56
        );
    }

    #[test]
    fn numbervalue_same_separators_errors() {
        assert!(numbervalue("1.234", Some('.'), Some('.')).is_err());
    }

    #[test]
    fn fixed_basic() {
        assert_eq!(fixed(1234.567, 2, false).unwrap(), "1,234.57");
        assert_eq!(fixed(1234.567, 2, true).unwrap(), "1234.57");
        // `decimals == 0` uses `format_integer`, which calls `f64::round`
        // (half-away-from-zero), so 1234.5 -> "1,235".
        assert_eq!(fixed(1234.5, 0, false).unwrap(), "1,235");
        // 1234.6 unambiguous up-round.
        assert_eq!(fixed(1234.6, 0, false).unwrap(), "1,235");
    }

    #[test]
    fn fixed_negative_decimals() {
        assert_eq!(fixed(12345.0, -2, false).unwrap(), "12,300");
        assert_eq!(fixed(12345.0, -2, true).unwrap(), "12300");
    }

    #[test]
    fn dollar_basic() {
        assert_eq!(dollar(1234.5, 2).unwrap(), "$1,234.50");
        assert_eq!(dollar(0.0, 2).unwrap(), "$0.00");
        assert_eq!(dollar(-1234.5, 2).unwrap(), "($1,234.50)");
        assert_eq!(dollar(99.0, 0).unwrap(), "$99");
    }

    #[test]
    fn value_vec_propagates_na() {
        let x = Character::from_options(vec![Some("12".into()), None, Some("nope".into())]);
        assert_eq!(value_vec(&x), vec![Some(12.0), None, None]);
    }
}
