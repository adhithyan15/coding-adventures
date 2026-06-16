//! # number-format-core — numeric format codes → display strings.
//!
//! A pure-Rust, no-I/O, WASM-compatible Layer-1 core (sibling of `text-core`,
//! `statistics-core`, …): it turns a computed number into the text a cell should
//! *show*, given an Excel / Lotus 1-2-3-style **format code** like `0.00`,
//! `#,##0`, `0%`, or `$#,##0.00;(#,##0.00)`. The spreadsheet engine computes the
//! value; this crate decides how it reads. Every frontend (web, SwiftUI, Qt,
//! Flutter, Compose, XAML) and chart renderer can share it.
//!
//! ## The format-code grammar (what a code means)
//!
//! A code is 1–3 `;`-separated **sections**, applied by the sign of the value:
//!
//! | sections        | positive | negative           | zero     |
//! |-----------------|----------|--------------------|----------|
//! | `pos`           | `pos`    | `pos` with `-`     | `pos`    |
//! | `pos;neg`       | `pos`    | `neg` (abs value)  | `pos`    |
//! | `pos;neg;zero`  | `pos`    | `neg` (abs value)  | `zero`   |
//!
//! Within a section the meaningful characters are:
//!
//! | token   | meaning                                                          |
//! |---------|------------------------------------------------------------------|
//! | `0`     | digit placeholder — always shown (pads with `0`)                 |
//! | `#`     | digit placeholder — shown only if significant                    |
//! | `?`     | like `#` (space-padding is approximated as `#` in v1)            |
//! | `.`     | decimal point (at most one per section)                          |
//! | `,`     | *between digits* → thousands grouping; *trailing* → ÷1000 each   |
//! | `%`     | scale ×100 and show a literal `%`                                |
//! | other   | a literal (e.g. `$`, `(`, `)`, spaces); `\x` and `"…"` escape    |
//!
//! Examples (see the tests for the exhaustive set):
//!
//! ```
//! use number_format_core::format_number;
//! assert_eq!(format_number(1234.5,  "#,##0.00"),        "1,234.50");
//! assert_eq!(format_number(0.0734,  "0.0%"),            "7.3%");
//! assert_eq!(format_number(-12.0,   "$#,##0.00;($#,##0.00)"), "($12.00)");
//! assert_eq!(format_number(2_600_000.0, "#,##0,, \"M\""), "3 M"); // scale ÷1e6, rounded
//! assert_eq!(format_number(42.0,    "General"),          "42");   // shortest repr
//! ```
//!
//! Rounding uses the standard library's correctly-rounded decimal formatting
//! (ties-to-even), so an exact half like `2.5` renders `"2"`; this differs from
//! Excel's ties-away-from-zero only on exact ties, which are rare with `f64`.
//!
//! ## Scope (v1)
//!
//! Numeric formats only. **Out of scope (documented follow-ups):** date/time
//! codes (`yyyy-mm-dd`, `hh:mm`), scientific notation (`0.00E+00`), fractions
//! (`# ?/?`), `[Color]` prefixes, and `[>100]`-style conditional sections. The
//! text (4th) section is also deferred — this crate formats numbers.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::fmt;

/// Cap on the fractional precision handed to the std formatter. An `f64` carries
/// no meaningful precision beyond this many decimal places, and the formatter
/// panics on a precision ≥ 65536 — so an adversarially long format code is
/// clamped here (the extra positions are trailing zeros, restored on output).
const MAX_FRACTION_DIGITS: usize = 340;

/// Why a format code could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatError {
    /// More than one decimal point in a single section (e.g. `0.0.0`).
    MultipleDecimalPoints,
    /// More than three `;`-separated sections.
    TooManySections,
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormatError::MultipleDecimalPoints => write!(f, "more than one decimal point in a section"),
            FormatError::TooManySections => write!(f, "more than three format sections"),
        }
    }
}

impl std::error::Error for FormatError {}

/// One `;`-separated section of a format code, pre-parsed into the numbers the
/// applier needs. `prefix`/`suffix` are the literal text around the digits.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Section {
    /// Literal text before the first digit placeholder (e.g. `$`, `(`).
    prefix: String,
    /// Literal text after the last digit placeholder (e.g. `%`, `)`).
    suffix: String,
    /// Count of `0` in the integer part — minimum integer digits (zero-padded).
    int_min: usize,
    /// Count of `0` after the decimal point — minimum fractional digits.
    frac_min: usize,
    /// Count of `0`/`#`/`?` after the decimal point — maximum fractional digits.
    frac_max: usize,
    /// A `,` appeared among the integer placeholders → group by thousands.
    grouping: bool,
    /// `%` appeared → multiply the value by 100.
    percent: bool,
    /// Trailing commas → divide the value by 1000 once per comma.
    scale_thousands: u32,
    /// Whether this section has any digit placeholders at all. A section that is
    /// pure literals (e.g. the negative section `"-"`) renders only its text.
    has_digits: bool,
}

/// A parsed numeric format code, ready to [`apply`](NumberFormat::apply) to any
/// number. Parse once, format many cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberFormat {
    /// 1–3 sections: positive, then optional negative, then optional zero.
    sections: Vec<Section>,
    /// `true` for the `General` (or empty) code — the shortest round-trip
    /// representation, ignoring sections.
    general: bool,
}

impl NumberFormat {
    /// Parse a format code. `"General"` (case-insensitive) or an empty string is
    /// the shortest-representation default.
    pub fn parse(code: &str) -> Result<NumberFormat, FormatError> {
        if code.is_empty() || code.eq_ignore_ascii_case("General") {
            return Ok(NumberFormat {
                sections: Vec::new(),
                general: true,
            });
        }
        let raw_sections = split_sections(code);
        if raw_sections.len() > 3 {
            return Err(FormatError::TooManySections);
        }
        let mut sections = Vec::with_capacity(raw_sections.len());
        for s in &raw_sections {
            sections.push(parse_section(s)?);
        }
        Ok(NumberFormat {
            sections,
            general: false,
        })
    }

    /// Format `value` to the display string this code prescribes.
    pub fn apply(&self, value: f64) -> String {
        if self.general {
            return general(value);
        }
        // Non-finite values have no sensible numeric rendering; mirror what a
        // spreadsheet shows for an out-of-band float.
        if value.is_nan() {
            return "NaN".to_string();
        }
        if value.is_infinite() {
            return if value < 0.0 { "-∞" } else { "∞" }.to_string();
        }

        // Defensive: `parse` always yields ≥1 section, but never index an empty
        // vec if a `NumberFormat` is ever constructed another way.
        if self.sections.is_empty() {
            return general(value);
        }

        let (section, force_minus) = self.select_section(value);
        if !section.has_digits {
            // A pure-literal section (e.g. a "-" negative section) shows its
            // text only; there are no digits to render.
            return format!("{}{}", section.prefix, section.suffix);
        }
        render(section, value, force_minus)
    }

    /// Pick the section for `value`'s sign, and whether the positive section
    /// must be prefixed with `-` (only when there is no dedicated negative
    /// section). Returns the section plus that flag.
    fn select_section(&self, value: f64) -> (&Section, bool) {
        let is_neg = value < 0.0;
        // Zero compares as neither positive nor negative here (`0.0 < 0.0` is
        // false), so it naturally routes to the positive/zero section.
        match self.sections.len() {
            0 => (&self.sections[0], false), // unreachable (parse yields ≥1), kept total
            1 => (&self.sections[0], is_neg),
            2 => {
                if is_neg {
                    (&self.sections[1], false) // dedicated negative section, abs value
                } else {
                    (&self.sections[0], false)
                }
            }
            _ => {
                if value > 0.0 {
                    (&self.sections[0], false)
                } else if value < 0.0 {
                    (&self.sections[1], false)
                } else {
                    (&self.sections[2], false) // zero section
                }
            }
        }
    }
}

/// Convenience: parse `code` and format `value` in one call. A malformed code
/// falls back to the shortest representation rather than erroring, so a host's
/// render path never panics on a bad user format.
pub fn format_number(value: f64, code: &str) -> String {
    match NumberFormat::parse(code) {
        Ok(fmt) => fmt.apply(value),
        Err(_) => general(value),
    }
}

/// The `General` format: the shortest decimal that round-trips, integers
/// without a trailing `.0`. Matches the engine's own `format_number` so a cell
/// with no explicit format renders identically.
fn general(value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    if value == value.trunc() && value.abs() < 1e16 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

/// Split a code into sections on `;`, honouring `\;` escapes and `"…;…"`
/// quoted literals (a `;` inside quotes or after a backslash is literal text,
/// not a section break).
fn split_sections(code: &str) -> Vec<String> {
    let mut sections = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    let mut escaped = false;
    for c in code.chars() {
        if escaped {
            current.push(c);
            escaped = false;
        } else if c == '\\' {
            current.push(c);
            escaped = true;
        } else if c == '"' {
            in_quote = !in_quote;
            current.push(c);
        } else if c == ';' && !in_quote {
            sections.push(std::mem::take(&mut current));
        } else {
            current.push(c);
        }
    }
    sections.push(current);
    sections
}

/// Parse one section's characters into a [`Section`]. Walks left to right,
/// classifying each char as a digit placeholder, structural token, or literal.
fn parse_section(code: &str) -> Result<Section, FormatError> {
    let mut s = Section::default();
    // Phase A — split the section into literal/placeholder atoms, resolving
    // quotes and backslash escapes so a quoted `0` is a literal, not a digit.
    enum Atom {
        Literal(char),
        Digit(char), // '0' | '#' | '?'
        Dot,
        Comma,
        Percent,
    }
    let mut atoms = Vec::new();
    let mut chars = code.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(next) = chars.next() {
                    atoms.push(Atom::Literal(next));
                }
            }
            '"' => {
                for q in chars.by_ref() {
                    if q == '"' {
                        break;
                    }
                    atoms.push(Atom::Literal(q));
                }
            }
            '0' | '#' | '?' => atoms.push(Atom::Digit(c)),
            '.' => atoms.push(Atom::Dot),
            ',' => atoms.push(Atom::Comma),
            '%' => atoms.push(Atom::Percent),
            other => atoms.push(Atom::Literal(other)),
        }
    }

    // Phase B — structural landmarks. Only digit placeholders and the decimal
    // point delimit the "number region"; literals before it are the prefix,
    // after it the suffix. A bare comma is never literal text — it is grouping,
    // scaling, or ignored.
    let dot_idx = atoms.iter().position(|a| matches!(a, Atom::Dot));
    let dot_at = dot_idx.unwrap_or(atoms.len());
    let first_num = atoms
        .iter()
        .position(|a| matches!(a, Atom::Digit(_) | Atom::Dot));
    let last_digit_idx = atoms.iter().rposition(|a| matches!(a, Atom::Digit(_)));

    // Validate: at most one decimal point per section.
    if atoms.iter().filter(|a| matches!(a, Atom::Dot)).count() > 1 {
        return Err(FormatError::MultipleDecimalPoints);
    }

    // Scaling commas: the contiguous run of commas immediately after the last
    // digit placeholder (e.g. `0,` ÷1000, `0.0,,` ÷1e6). Record their indices so
    // they aren't also emitted as literals.
    let mut scaling_idx = std::collections::HashSet::new();
    if let Some(ld) = last_digit_idx {
        for (j, atom) in atoms.iter().enumerate().skip(ld + 1) {
            match atom {
                Atom::Comma => {
                    s.scale_thousands += 1;
                    scaling_idx.insert(j);
                }
                _ => break,
            }
        }
    }

    let mut seen_dot = false;
    for (i, atom) in atoms.iter().enumerate() {
        let is_prefix = matches!(first_num, Some(fnum) if i < fnum) || first_num.is_none();
        match atom {
            Atom::Literal(c) => {
                if is_prefix {
                    s.prefix.push(*c);
                } else {
                    s.suffix.push(*c);
                }
            }
            Atom::Percent => {
                s.percent = true;
                if is_prefix {
                    s.prefix.push('%');
                } else {
                    s.suffix.push('%');
                }
            }
            Atom::Dot => {
                seen_dot = true;
                s.has_digits = true;
            }
            Atom::Digit(d) => {
                s.has_digits = true;
                if seen_dot {
                    s.frac_max += 1;
                    if *d == '0' {
                        s.frac_min += 1;
                    }
                } else if *d == '0' {
                    s.int_min += 1;
                }
            }
            Atom::Comma => {
                if scaling_idx.contains(&i) {
                    // Already counted as a trailing scaling comma.
                } else if i < dot_at && atoms[i + 1..dot_at].iter().any(|a| matches!(a, Atom::Digit(_))) {
                    // A digit placeholder follows in the integer part → grouping.
                    s.grouping = true;
                }
                // Any other bare comma (e.g. in the fractional part) is ignored,
                // matching Excel — never rendered as a literal.
            }
        }
    }
    Ok(s)
}

/// Render a numeric section for `value`. `force_minus` prepends `-` (used when a
/// single-section code formats a negative with no dedicated negative section).
fn render(s: &Section, value: f64, force_minus: bool) -> String {
    // Scale: percent ×100, trailing commas ÷1000 each.
    let mut v = value.abs();
    if s.percent {
        v *= 100.0;
    }
    for _ in 0..s.scale_thousands {
        v /= 1000.0;
    }

    // Round to the maximum fractional digits using the standard library's
    // correctly-rounded decimal formatting, then split into integer/fraction.
    //
    // SECURITY: `frac_max` is attacker-controlled (the count of `#`/`0` after the
    // decimal in a user-typed code). The std formatter stores precision as a
    // `u16`, so a precision ≥ 65536 PANICS. An `f64` carries no meaningful
    // precision beyond ~340 decimal places regardless, so we cap there: every
    // digit past it would be a trailing zero, which the pad/trim logic below
    // restores. This keeps the documented "never panics on a bad format" promise.
    let prec = s.frac_max.min(MAX_FRACTION_DIGITS);
    let rounded = format!("{:.*}", prec, v);
    let (int_part, frac_part) = match rounded.split_once('.') {
        Some((i, f)) => (i.to_string(), f.to_string()),
        None => (rounded, String::new()),
    };

    // Integer: pad to the minimum digit count, then group by thousands.
    let mut int_digits = int_part.trim_start_matches('0').to_string();
    if int_digits.is_empty() {
        int_digits = "0".repeat(s.int_min.max(if s.frac_max == 0 { 1 } else { 0 }));
        // If there is a fractional part we may legitimately show ".5" with a
        // bare leading 0 only when int_min ≥ 1; Excel shows "0.5" for "0.0",
        // ".5" for "#.0". int_min already encodes that.
    }
    while int_digits.len() < s.int_min {
        int_digits.insert(0, '0');
    }
    if s.grouping && !int_digits.is_empty() {
        int_digits = group_thousands(&int_digits);
    }

    // Fraction: trim trailing zeros down to the minimum, then (if the code
    // demanded more required digits than we computed because precision was
    // capped) pad back out with zeros to the required minimum.
    let mut frac_digits = frac_part;
    while frac_digits.len() > s.frac_min && frac_digits.ends_with('0') {
        frac_digits.pop();
    }
    while frac_digits.len() < s.frac_min {
        frac_digits.push('0');
    }

    let mut out = String::new();
    out.push_str(&s.prefix);
    if force_minus {
        out.push('-');
    }
    out.push_str(&int_digits);
    if !frac_digits.is_empty() {
        out.push('.');
        out.push_str(&frac_digits);
    }
    out.push_str(&s.suffix);
    out
}

/// Insert `,` every three digits from the right of a run of digits.
// `% 3 == 0` reads more clearly here than `.is_multiple_of(3)`, and avoids
// depending on that newer method's availability across toolchains.
#[allow(clippy::manual_is_multiple_of)]
fn group_thousands(digits: &str) -> String {
    let bytes = digits.as_bytes();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    let n = bytes.len();
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (n - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*b as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(v: f64, code: &str) -> String {
        format_number(v, code)
    }

    // ── General / defaults ──────────────────────────────────────────
    #[test]
    fn general_is_shortest_repr() {
        assert_eq!(f(42.0, "General"), "42");
        assert_eq!(f(42.0, ""), "42");
        assert_eq!(f(1.25, "general"), "1.25");
        assert_eq!(f(0.0, "General"), "0");
        assert_eq!(f(-7.0, "General"), "-7");
    }

    // ── Fixed decimals + zero padding ───────────────────────────────
    #[test]
    fn fixed_decimals_round_and_pad() {
        assert_eq!(f(1.23456, "0.00"), "1.23");
        assert_eq!(f(3.0, "0.00"), "3.00");
        assert_eq!(f(3.5, "0"), "4"); // rounds to integer
        assert_eq!(f(0.5, "0.0"), "0.5");
        assert_eq!(f(7.0, "000"), "007"); // leading-zero pad
    }

    #[test]
    fn hash_placeholders_are_optional() {
        assert_eq!(f(3.5, "#.##"), "3.5"); // trailing optional zero dropped
        assert_eq!(f(3.0, "#.##"), "3"); // whole fraction dropped
        assert_eq!(f(0.5, "#.##"), ".5"); // no required integer digit → ".5"
        assert_eq!(f(0.5, "0.##"), "0.5"); // required integer digit → "0.5"
    }

    // ── Thousands grouping ──────────────────────────────────────────
    #[test]
    fn thousands_grouping() {
        assert_eq!(f(1234.5, "#,##0.00"), "1,234.50");
        assert_eq!(f(1234567.0, "#,##0"), "1,234,567");
        assert_eq!(f(999.0, "#,##0"), "999");
        assert_eq!(f(1000.0, "#,##0"), "1,000");
    }

    // ── Percent ─────────────────────────────────────────────────────
    #[test]
    fn percent_scales_and_shows_sign() {
        assert_eq!(f(0.0734, "0.0%"), "7.3%");
        assert_eq!(f(0.5, "0%"), "50%");
        assert_eq!(f(1.0, "0%"), "100%");
    }

    // ── Trailing-comma scaling ──────────────────────────────────────
    #[test]
    fn trailing_comma_scales_by_thousands() {
        assert_eq!(f(1_500.0, "0,"), "2"); // ÷1000, rounded
        assert_eq!(f(2_500_000.0, "0.0,,"), "2.5"); // ÷1e6
        assert_eq!(f(1234.0, "#,##0.0,"), "1.2"); // grouping + scale combine
    }

    // ── Currency prefix + literal suffix ────────────────────────────
    #[test]
    fn prefix_and_suffix_literals() {
        assert_eq!(f(12.0, "$#,##0.00"), "$12.00");
        assert_eq!(f(5.0, "0 \"kg\""), "5 kg");
        assert_eq!(f(5.0, "0\\x"), "5x"); // backslash escape
    }

    // ── Negative handling ───────────────────────────────────────────
    #[test]
    fn negative_without_section_gets_minus() {
        assert_eq!(f(-12.5, "0.0"), "-12.5");
        assert_eq!(f(-1234.0, "#,##0"), "-1,234");
        assert_eq!(f(-0.25, "0%"), "-25%");
    }

    #[test]
    fn negative_section_uses_abs_value_and_its_own_literals() {
        // Parentheses-negative accounting style.
        assert_eq!(f(-12.0, "$#,##0.00;($#,##0.00)"), "($12.00)");
        assert_eq!(f(12.0, "$#,##0.00;($#,##0.00)"), "$12.00");
        // A bare "-" negative section.
        assert_eq!(f(-5.0, "0.0;\\(0.0\\)"), "(5.0)");
    }

    #[test]
    fn three_sections_route_zero_separately() {
        let code = "0.0;(0.0);\"zero\"";
        assert_eq!(f(5.0, code), "5.0");
        assert_eq!(f(-5.0, code), "(5.0)");
        assert_eq!(f(0.0, code), "zero");
    }

    // ── Rounding edges ──────────────────────────────────────────────
    #[test]
    fn rounding_carries_into_integer_and_grouping() {
        assert_eq!(f(999.99, "#,##0.0"), "1,000.0"); // carry ripples through grouping
        assert_eq!(f(9.96, "0.0"), "10.0");
    }

    // ── Robustness ──────────────────────────────────────────────────
    #[test]
    fn malformed_code_falls_back_to_general() {
        // Two decimal points is a parse error → General fallback, never a panic.
        assert_eq!(f(1.25, "0.0.0"), "1.25");
    }

    #[test]
    fn too_many_sections_is_an_error() {
        assert_eq!(
            NumberFormat::parse("0;0;0;0"),
            Err(FormatError::TooManySections)
        );
    }

    #[test]
    fn adversarial_huge_fraction_does_not_panic() {
        // A format code with > 65 536 fractional placeholders would overflow the
        // std formatter's u16 precision and panic. The precision cap must keep
        // the no-panic promise; required `0`s still pad out to the demanded width.
        let many_hash = format!("0.{}", "#".repeat(70_000));
        assert_eq!(f(1.5, &many_hash), "1.5"); // optional digits collapse
        let many_zero = format!("0.{}", "0".repeat(70_000));
        let out = f(1.5, &many_zero);
        assert!(out.starts_with("1.5"));
        assert_eq!(out.len(), "1.".len() + 70_000); // padded to the required width
    }

    #[test]
    fn non_finite_values_do_not_panic() {
        assert_eq!(f(f64::NAN, "0.00"), "NaN");
        assert_eq!(f(f64::INFINITY, "0.00"), "∞");
        assert_eq!(f(f64::NEG_INFINITY, "0.00"), "-∞");
    }

    #[test]
    fn parse_then_apply_many() {
        let fmt = NumberFormat::parse("#,##0.00").unwrap();
        assert_eq!(fmt.apply(1.0), "1.00");
        assert_eq!(fmt.apply(1234.5), "1,234.50");
        assert_eq!(fmt.apply(-1234.5), "-1,234.50");
    }
}
