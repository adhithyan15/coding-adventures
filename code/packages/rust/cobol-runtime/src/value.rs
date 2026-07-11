//! Storage values and the exact `MOVE` receiving transforms.
//!
//! A numeric value in flight is a [`Decimal`] — a sign and two digit strings
//! (integer and fractional parts). `MOVE` reshapes it (or a character string)
//! into a receiver's fixed picture using COBOL's precise justify / pad /
//! truncate rules. These are pure functions of strings, so they are trivially
//! unit-tested against the standard's behaviour.

/// A numeric value as digit strings — the shape needed to align by the decimal
/// point when moving into a numeric receiver. (v0.1 is unsigned; `neg` is
/// carried for the next PR and currently always `false`.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decimal {
    pub neg: bool,
    /// Integer-part digits, most-significant first (may be empty → treated as 0).
    pub int: String,
    /// Fractional-part digits, most-significant first.
    pub frac: String,
}

impl Decimal {
    /// Zero.
    pub fn zero() -> Decimal {
        Decimal { neg: false, int: "0".into(), frac: String::new() }
    }

    /// Parse a numeric literal such as `"42"`, `"-3"`, `"3.14"`, `".5"`.
    /// Returns `None` if it is not a numeric literal.
    pub fn parse_literal(s: &str) -> Option<Decimal> {
        let mut chars = s.chars().peekable();
        let mut neg = false;
        match chars.peek() {
            Some('+') => { chars.next(); }
            Some('-') => { neg = true; chars.next(); }
            _ => {}
        }
        let rest: String = chars.collect();
        let mut parts = rest.splitn(2, '.');
        let int = parts.next().unwrap_or("");
        let frac = parts.next().unwrap_or("");
        if int.is_empty() && frac.is_empty() {
            return None;
        }
        if !int.chars().all(|c| c.is_ascii_digit()) || !frac.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        Some(Decimal { neg, int: int.to_string(), frac: frac.to_string() })
    }

    /// The digit characters this value occupies, for display of a numeric item.
    pub fn digits(&self) -> String {
        format!("{}{}", self.int, self.frac)
    }
}

/// Move a numeric value into a numeric receiver of `int_digits` integer
/// positions and `dec_digits` fractional positions. Returns the receiver's
/// stored digit characters (length `int_digits + dec_digits`).
///
/// Alignment is by the decimal point: the integer part is **right-justified**
/// (zero-filled left, high-order digits truncated on overflow); the fractional
/// part is **left-justified** (zero-filled right, low-order digits truncated).
pub fn move_into_numeric(src: &Decimal, int_digits: usize, dec_digits: usize) -> String {
    // Integer part: keep the low-order `int_digits` (truncate high-order),
    // then left-pad with zeros.
    let int_src = if src.int.is_empty() { "0" } else { &src.int };
    let int_kept: String = if int_src.len() > int_digits {
        int_src[int_src.len() - int_digits..].to_string()
    } else {
        format!("{:0>width$}", int_src, width = int_digits)
    };

    // Fractional part: keep the high-order `dec_digits` (truncate low-order),
    // then right-pad with zeros.
    let frac_kept: String = if src.frac.len() > dec_digits {
        src.frac[..dec_digits].to_string()
    } else {
        format!("{:0<width$}", src.frac, width = dec_digits)
    };

    format!("{int_kept}{frac_kept}")
}

/// Move a character string into a character receiver of `size` positions.
/// The source is placed **left-justified**; a shorter source is
/// **space-padded on the right**, a longer source is **truncated on the right**.
pub fn move_into_char(src: &str, size: usize) -> String {
    let chars: Vec<char> = src.chars().collect();
    if chars.len() >= size {
        chars[..size].iter().collect()
    } else {
        let mut s: String = chars.iter().collect();
        s.extend(std::iter::repeat(' ').take(size - chars.len()));
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_literal_parsing() {
        assert_eq!(Decimal::parse_literal("42"), Some(Decimal { neg: false, int: "42".into(), frac: "".into() }));
        assert_eq!(Decimal::parse_literal("3.14"), Some(Decimal { neg: false, int: "3".into(), frac: "14".into() }));
        assert_eq!(Decimal::parse_literal("-3"), Some(Decimal { neg: true, int: "3".into(), frac: "".into() }));
        assert_eq!(Decimal::parse_literal("HELLO"), None);
    }

    #[test]
    fn numeric_move_zero_fills_and_right_justifies() {
        // MOVE 42 TO PIC 9(5) → "00042"
        let d = Decimal::parse_literal("42").unwrap();
        assert_eq!(move_into_numeric(&d, 5, 0), "00042");
    }

    #[test]
    fn numeric_move_truncates_high_and_low_order() {
        // MOVE 123.456 TO PIC 9(2)V9 → integer keeps "23", fraction keeps "4"
        let d = Decimal::parse_literal("123.456").unwrap();
        assert_eq!(move_into_numeric(&d, 2, 1), "234");
    }

    #[test]
    fn numeric_move_pads_fraction() {
        // MOVE 7 TO PIC 9(3)V99 → "00700"
        let d = Decimal::parse_literal("7").unwrap();
        assert_eq!(move_into_numeric(&d, 3, 2), "00700");
    }

    #[test]
    fn char_move_pads_and_truncates() {
        assert_eq!(move_into_char("HI", 5), "HI   ");
        assert_eq!(move_into_char("HELLO WORLD", 5), "HELLO");
        assert_eq!(move_into_char("EXACT", 5), "EXACT");
    }
}
