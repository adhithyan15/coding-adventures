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

    /// Whether this value is zero (all digit positions are `0`).
    pub fn is_zero(&self) -> bool {
        self.int.chars().chain(self.frac.chars()).all(|c| c == '0')
    }

    /// Compare two decimals by numeric value. Works on the digit strings (not
    /// `i128`) so it is exact for numbers of any size. `-0` compares equal to
    /// `+0`.
    pub fn cmp_value(&self, other: &Decimal) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        let neg_self = self.neg && !self.is_zero();
        let neg_other = other.neg && !other.is_zero();
        match (neg_self, neg_other) {
            (false, true) => return Ordering::Greater,
            (true, false) => return Ordering::Less,
            _ => {}
        }
        let mag = magnitude_cmp(self, other);
        // If both are negative, the larger magnitude is the smaller value.
        if neg_self {
            mag.reverse()
        } else {
            mag
        }
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

/// Fixed-point division `num / den`, truncated (toward zero, as COBOL does
/// without `ROUNDED`) to exactly `result_scale` fractional digits.
///
/// Returns `None` on `i128` overflow of the intermediate scaling; the caller is
/// expected to have already rejected a zero divisor (`den == 0` here also yields
/// `None`, but a zero divisor should surface as [`RuntimeError::DivideByZero`]).
///
/// Derivation: with `num = N/10^sn` and `den = D/10^sd`, the value scaled by
/// `10^result_scale` is `floor( (N · 10^(sd + result_scale)) / (D · 10^sn) )`.
pub fn div(num: &Decimal, den: &Decimal, result_scale: usize) -> Option<Decimal> {
    let (sn, sd) = (num.frac.len(), den.frac.len());
    let n = num.to_scaled(sn)?;
    let d = den.to_scaled(sd)?;
    if d == 0 {
        return None;
    }
    // Scale the numerator up and the denominator up so the quotient carries
    // `result_scale` fractional digits.
    let numerator = n.checked_mul(pow10(sd + result_scale)?)?;
    let denominator = d.checked_mul(pow10(sn)?)?;
    // i128 division truncates toward zero — exactly COBOL's un-rounded behaviour.
    let scaled = numerator.checked_div(denominator)?;
    Some(Decimal::from_scaled(scaled, result_scale))
}

/// `10^exp` as `i128`, or `None` on overflow.
fn pow10(exp: usize) -> Option<i128> {
    let mut v: i128 = 1;
    for _ in 0..exp {
        v = v.checked_mul(10)?;
    }
    Some(v)
}

/// Compare the unsigned magnitudes of two decimals: integer part first (by
/// significant length, then digit-by-digit), then the fractional part
/// (zero-padded to equal length).
fn magnitude_cmp(a: &Decimal, b: &Decimal) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let ai = a.int.trim_start_matches('0');
    let bi = b.int.trim_start_matches('0');
    match ai.len().cmp(&bi.len()) {
        Ordering::Equal => {}
        o => return o,
    }
    match ai.cmp(bi) {
        Ordering::Equal => {}
        o => return o,
    }
    let width = a.frac.len().max(b.frac.len());
    let af = format!("{:0<width$}", a.frac);
    let bf = format!("{:0<width$}", b.frac);
    af.cmp(&bf)
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

    #[test]
    fn division_truncates_toward_zero() {
        // 10 / 4 = 2.5 → to 0 decimals truncates to 2.
        assert_eq!(div(&d("10"), &d("4"), 0).unwrap(), d("2"));
        // 10 / 3 = 3.333… → to 2 decimals truncates to 3.33.
        assert_eq!(div(&d("10"), &d("3"), 2).unwrap(), d("3.33"));
        // Exact: 9 / 3 = 3.00 at 2 decimals.
        assert_eq!(div(&d("9"), &d("3"), 2).unwrap(), d("3.00"));
    }

    #[test]
    fn division_of_fractional_operands() {
        // 7.5 / 2.5 = 3.0
        assert_eq!(div(&d("7.5"), &d("2.5"), 1).unwrap(), d("3.0"));
    }

    #[test]
    fn division_by_zero_returns_none() {
        assert_eq!(div(&d("5"), &d("0"), 2), None);
    }

    #[test]
    fn decimal_comparison() {
        use std::cmp::Ordering::*;
        assert_eq!(d("5").cmp_value(&d("3")), Greater);
        assert_eq!(d("3").cmp_value(&d("5")), Less);
        assert_eq!(d("42").cmp_value(&d("42")), Equal);
        // Different fraction lengths, same value.
        assert_eq!(d("1.5").cmp_value(&d("1.50")), Equal);
        assert_eq!(d("1.50").cmp_value(&d("1.5")), Equal);
        // Magnitude by significant integer length.
        assert_eq!(d("100").cmp_value(&d("99")), Greater);
        assert_eq!(d("007").cmp_value(&d("7")), Equal);
        // Signs: negatives order below positives; -0 == +0.
        assert_eq!(d("-2").cmp_value(&d("3")), Less);
        assert_eq!(d("-2").cmp_value(&d("-5")), Greater);
        assert_eq!(d("-0").cmp_value(&d("0")), Equal);
    }
}
