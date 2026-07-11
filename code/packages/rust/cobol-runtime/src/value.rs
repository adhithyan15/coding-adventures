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

    /// This value as a signed `i128` scaled by `10^scale` (i.e. with exactly
    /// `scale` fractional digits). Returns `None` if it overflows `i128`
    /// (~38 digits — beyond any real COBOL numeric field) or if truncating the
    /// fraction to `scale` digits would lose precision (callers pick a `scale`
    /// large enough that it never does). This is the integer form on which exact
    /// fixed-point add/subtract/multiply are performed.
    fn to_scaled(&self, scale: usize) -> Option<i128> {
        if self.frac.len() > scale {
            return None;
        }
        let int = if self.int.is_empty() { "0" } else { self.int.as_str() };
        let frac = format!("{:0<width$}", self.frac, width = scale);
        let combined = format!("{int}{frac}");
        let mag: i128 = combined.parse().ok()?;
        Some(if self.neg { -mag } else { mag })
    }

    /// Rebuild a [`Decimal`] from a signed `i128` scaled by `10^scale`.
    fn from_scaled(v: i128, scale: usize) -> Decimal {
        let neg = v < 0;
        // Zero-pad so there is always at least one integer digit.
        let s = format!("{:0>width$}", v.unsigned_abs(), width = scale + 1);
        let split = s.len() - scale;
        Decimal { neg, int: s[..split].to_string(), frac: s[split..].to_string() }
    }
}

/// Exact fixed-point addition. The result keeps the larger of the two operands'
/// fractional lengths, so no precision is lost. `None` on `i128` overflow.
pub fn add(a: &Decimal, b: &Decimal) -> Option<Decimal> {
    let scale = a.frac.len().max(b.frac.len());
    let r = a.to_scaled(scale)?.checked_add(b.to_scaled(scale)?)?;
    Some(Decimal::from_scaled(r, scale))
}

/// Exact fixed-point subtraction (`a - b`).
pub fn sub(a: &Decimal, b: &Decimal) -> Option<Decimal> {
    let scale = a.frac.len().max(b.frac.len());
    let r = a.to_scaled(scale)?.checked_sub(b.to_scaled(scale)?)?;
    Some(Decimal::from_scaled(r, scale))
}

/// Exact fixed-point multiplication. The result's fractional length is the sum
/// of the operands' — the standard COBOL composite before truncation into a
/// receiver.
pub fn mul(a: &Decimal, b: &Decimal) -> Option<Decimal> {
    let (sa, sb) = (a.frac.len(), b.frac.len());
    let r = a.to_scaled(sa)?.checked_mul(b.to_scaled(sb)?)?;
    Some(Decimal::from_scaled(r, sa + sb))
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

    // ----------------------------------------------------------------------
    // Fixed-point decimal arithmetic
    // ----------------------------------------------------------------------

    fn d(s: &str) -> Decimal {
        Decimal::parse_literal(s).unwrap()
    }

    #[test]
    fn decimal_add_aligns_by_the_point() {
        // 1.5 + 2.25 = 3.75 (result keeps the wider fraction)
        assert_eq!(add(&d("1.5"), &d("2.25")).unwrap(), d("3.75"));
        // 7 + 8 = 15
        assert_eq!(add(&d("7"), &d("8")).unwrap(), d("15"));
    }

    #[test]
    fn decimal_sub_can_go_negative() {
        // 3 - 5 = -2 (sign carried; an unsigned receiver later drops it)
        assert_eq!(sub(&d("3"), &d("5")).unwrap(), d("-2"));
        assert_eq!(sub(&d("10.00"), &d("0.25")).unwrap(), d("9.75"));
    }

    #[test]
    fn decimal_mul_sums_fraction_lengths() {
        // 1.5 * 2.5 = 3.75; fraction length = 1 + 1 = 2
        let r = mul(&d("1.5"), &d("2.5")).unwrap();
        assert_eq!(r, Decimal { neg: false, int: "3".into(), frac: "75".into() });
        // 12 * 12 = 144
        assert_eq!(mul(&d("12"), &d("12")).unwrap(), d("144"));
    }

    #[test]
    fn arithmetic_result_moves_into_receiver_truncating() {
        // (2.5 * 2.5 = 6.25) moved into PIC 9(3)V9 → "0062" (low-order truncated).
        let r = mul(&d("2.5"), &d("2.5")).unwrap();
        assert_eq!(move_into_numeric(&r, 3, 1), "0062");
    }
}
