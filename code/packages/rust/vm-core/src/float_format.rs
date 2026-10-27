//! Shortest round-trip float digits — the VM's (and the JIT's) lowering of
//! the LANG83 PR7b ops `f64_class`, `f64_digits` and `f64_exp10`.
//!
//! # Two questions: which digits, and where the point goes
//!
//! Printing a float is two separate decisions.
//!
//! 1. **The digits.** Python's `repr(x)`, JavaScript's `String(x)`, Java 19+
//!    and Ryu all print the *shortest* decimal digit string that reads back
//!    as exactly `x`. If several strings of that length read back as `x`, they
//!    print the one *closest* to `x`; on an exact tie, the one whose last
//!    digit is even. `0.1` is really `0.1000000000000000055511151231257827…`,
//!    but `0.1` is the shortest string that rounds back to it. This rule has
//!    one definition that names no language, so it is an IIR op.
//!
//! 2. **The layout.** Given digits `D` and the decimal exponent `e10` of the
//!    first digit (`123.4` → `D = 1234`, `e10 = 2`), every language places
//!    the point, the exponent and the sign its own way:
//!
//!    | value | Python `repr` | JavaScript `toString` |
//!    |-------|---------------|-----------------------|
//!    | `1e16` | `1e+16` | `10000000000000000` |
//!    | `100.0` | `100.0` | `100` |
//!    | `-0.0` | `-0.0` | `0` |
//!
//!    That is a language's rule, so it is **not** here. Each compiler emits a
//!    module-local helper that lays the digits out with integer ops and
//!    `putchar` (python-iir-compiler's `__py_write_float`,
//!    javascript-iir-compiler's `__js_write_number`).
//!
//! Until LANG83 PR7e this module also held both layouts (`format_f64`,
//! `python_repr`, `js_to_string`) for the VM's `write_f64_py` /
//! `write_f64_js` builtins. Those builtins are gone, and with them the
//! layouts: no line of the VM knows which language it runs.
//!
//! # Where the digits come from here
//!
//! Rust's `{:e}` formatting prints the shortest round-trip digits, closest to
//! the value — with ONE difference: when the value lies *exactly* halfway
//! between two shortest candidates, Rust takes the upper one, while Python,
//! JavaScript (ECMA-262 `Number::toString`, step 5's "choose the even one"),
//! Java 19+ and Ryu all take the one whose last digit is even. That is not
//! exotic: `2^-25` is exactly `2.98023223876953125e-08`, whose shortest
//! digits are `29802322387695312` (even), where Rust prints
//! `2.9802322387695313e-8`; so does any double near `1e15` with a `.25` or
//! `.75` fraction (`1072012167405248.25`). Measured against host `python3` on
//! 535 432 values, Rust's digits differed on 94 — every one of them such a
//! tie.
//!
//! So this module reads Rust's shortest digits out of `format!("{:e}", x)`
//! (`"1.2345e-7"`, `"1e16"`), and then checks for that one case with Rust's
//! *exact* formatting (`{:.1100e}` prints every digit of the binary value —
//! a double has at most 767 significant decimal digits): if the exact
//! expansion is one digit longer than the shortest digits and ends in `5`, the
//! value is a tie, and the even neighbour is taken whenever it, too, reads back
//! as the same double. Both answers come from Rust's own formatting.
//!
//! The VM's `f64_class` / `f64_digits` / `f64_exp10` handlers and the JIT's
//! `CLASS_F64` / `DIGITS_F64` / `EXP10_F64` bytecodes both call these
//! functions, so the two in-process columns cannot disagree.

/// The shortest round-trip digits of a finite, non-zero `x`, as a digit
/// STRING: `(digits, e10)` with `|x| ≈ 0.d₁ × 10^(e10+1)`, i.e. `e10` is the
/// decimal exponent of the first digit. `digits` has no leading or trailing
/// zeros.
///
/// The internal core; [`shortest_digits`] (the LANG83 PR7b `f64_digits` /
/// `f64_exp10` answer) reads it as an integer.
fn shortest_digit_string(x: f64) -> (String, i32) {
    debug_assert!(x.is_finite() && x != 0.0, "shortest_digit_string needs a finite non-zero value");
    // `{:e}` of a positive float is `d[.ddd]e[-]n`: the mantissa's digits ARE
    // the shortest digits, and `n` is the exponent of the first one.
    let (digits, e10) = split_sci(&format!("{:e}", x.abs()));
    match even_tie_neighbour(x.abs(), &digits, e10) {
        Some(even) => even,
        None => (digits, e10),
    }
}

/// LANG83 PR7b — the shortest round-trip digits of a finite, non-zero `x` as
/// integers: `(D, e10)`, where `D` is the significand with no trailing zeros
/// (`1 <= D < 10^17`) and `e10` the exponent of its leading digit, so
/// `|x| = D × 10^(e10 − n + 1)` with `n` the number of digits of `D`.
///
/// ```text
///   x         digits     D       e10     check
///   123.456   "123456"   123456  2       123456 × 10^(2−6+1) = 123.456
///   -0.001    "1"        1       -3      1 × 10^-3           (the sign is not part of it)
///   1e16      "1"        1       16
///   5e-324    "5"        5       -324    the smallest subnormal
/// ```
///
/// At most 17 digits, so `D` always fits in a `u64` (and an `i64`).
///
/// ```
/// use vm_core::float_format::shortest_digits;
/// assert_eq!(shortest_digits(123.456), (123456, 2));
/// assert_eq!(shortest_digits(-0.001), (1, -3));
/// assert_eq!(shortest_digits(1e16), (1, 16));
/// assert_eq!(shortest_digits(5e-324), (5, -324));
/// ```
pub fn shortest_digits(x: f64) -> (u64, i32) {
    let (digits, e10) = shortest_digit_string(x);
    // `digits` is 1..=17 ASCII decimal digits, so neither the parse nor the
    // u64 can fail; `unwrap_or(0)` only keeps a malformed input from panicking.
    (digits.parse::<u64>().unwrap_or(0), e10)
}

/// LANG83 PR7b `f64_class`: `0` for `±0`, `1` for a finite non-zero value
/// (subnormals included), `2` for `±Infinity`, `3` for NaN.
///
/// ```
/// use vm_core::float_format::f64_class;
/// assert_eq!([f64_class(-0.0), f64_class(5e-324), f64_class(f64::NEG_INFINITY), f64_class(f64::NAN)], [0, 1, 2, 3]);
/// ```
pub fn f64_class(x: f64) -> i64 {
    use std::num::FpCategory::{Infinite, Nan, Normal, Subnormal, Zero};
    match x.classify() {
        Zero => 0,
        Normal | Subnormal => 1,
        Infinite => 2,
        Nan => 3,
    }
}

/// LANG83 PR7b `f64_digits`: `D` of [`shortest_digits`] for class 1, else `0`.
///
/// ```
/// use vm_core::float_format::f64_digits;
/// assert_eq!(f64_digits(1.5), 15);
/// assert_eq!(f64_digits(-2.5), 25);
/// assert_eq!(f64_digits(f64::INFINITY), 0);
/// ```
pub fn f64_digits(x: f64) -> i64 {
    if f64_class(x) == 1 {
        shortest_digits(x).0 as i64
    } else {
        0
    }
}

/// LANG83 PR7b `f64_exp10`: `e10` of [`shortest_digits`] for class 1, else `0`.
///
/// ```
/// use vm_core::float_format::f64_exp10;
/// assert_eq!(f64_exp10(1.5), 0);
/// assert_eq!(f64_exp10(1e21), 21);
/// assert_eq!(f64_exp10(5e-324), -324);
/// assert_eq!(f64_exp10(0.0), 0);
/// ```
pub fn f64_exp10(x: f64) -> i64 {
    if f64_class(x) == 1 {
        i64::from(shortest_digits(x).1)
    } else {
        0
    }
}

/// Split Rust's `d[.ddd]e[-]n` into its digit string (trailing zeros removed)
/// and the exponent `n`.
fn split_sci(sci: &str) -> (String, i32) {
    let (mantissa, exponent) = sci.split_once('e').unwrap_or((sci, "0"));
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let trimmed = digits.trim_end_matches('0');
    let digits = if trimmed.is_empty() { "0".to_string() } else { trimmed.to_string() };
    (digits, exponent.parse::<i32>().unwrap_or(0))
}

/// `true` if the decimal `0.<digits> × 10^(e10+1)` reads back as exactly `x`.
fn reads_back_as(digits: &str, e10: i32, x: f64) -> bool {
    format!("0.{digits}e{}", e10 + 1).parse::<f64>().is_ok_and(|y| y.to_bits() == x.to_bits())
}

/// Rust's shortest mode rounds an exact tie up. If `digits` (odd last digit)
/// is one side of an exact tie, return the other, even, side when it also
/// reads back as `x` — the round-half-even answer Python and JavaScript give.
///
/// ```text
/// x      = 2^-25 = 2.98023223876953125e-08     exact expansion, 18 digits, ends in 5
/// Rust   = 29802322387695313                   the upper neighbour (odd)
/// lower  = 29802322387695312                   even, and reads back as x  → taken
/// ```
fn even_tie_neighbour(x: f64, digits: &str, e10: i32) -> Option<(String, i32)> {
    let last = *digits.as_bytes().last()?;
    if (last - b'0').is_multiple_of(2) {
        return None; // already even: a tie, if any, was already broken our way
    }
    let (exact, exact_e10) = split_sci(&format!("{:.1100e}", x));
    if exact_e10 != e10 || exact.len() != digits.len() + 1 || !exact.ends_with('5') {
        return None; // not exactly halfway between two candidates of this length
    }
    // The two candidates either side of the tie: `lower` and `lower + 1`.
    let lower = &exact[..digits.len()];
    let candidate = if lower == digits {
        // Rust took the lower side and it is odd; the upper side is even.
        increment_digits(lower, e10)
    } else {
        (lower.to_string(), e10)
    };
    let (cand_digits, cand_e10) = candidate;
    let trimmed = cand_digits.trim_end_matches('0');
    let cand = (if trimmed.is_empty() { "0".to_string() } else { trimmed.to_string() }, cand_e10);
    reads_back_as(&cand.0, cand.1, x).then_some(cand)
}

/// `digits + 1` in the last place (`"1299"` → `"13"` after trimming;
/// `"999"` → `"1"` with the exponent one higher).
fn increment_digits(digits: &str, e10: i32) -> (String, i32) {
    let mut bytes = digits.as_bytes().to_vec();
    for b in bytes.iter_mut().rev() {
        if *b == b'9' {
            *b = b'0';
        } else {
            *b += 1;
            return (String::from_utf8(bytes).unwrap_or_default(), e10);
        }
    }
    // Every digit carried: 999 + 1 = 1000.
    ("1".to_string(), e10 + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `(value, D, e10)` — every row is host `python3`'s `repr` split at the
    /// digits (the same values the PR6a layout tables used: each layout edge,
    /// the extremes and a tie).
    const TABLE: &[(f64, u64, i32)] = &[
        (1.0, 1, 0),
        (100.0, 1, 2),
        (0.1, 1, -1),
        (123.456, 123456, 2),
        (1e15, 1, 15),
        (1e16, 1, 16),
        (1e20, 1, 20),
        (1e21, 1, 21),
        (1e-4, 1, -4),
        (1e-5, 1, -5),
        (1e-6, 1, -6),
        (1e-7, 1, -7),
        (1.5e-7, 15, -7),
        (-2.5, 25, 0),
        (5e-324, 5, -324),
        (2.2250738585072014e-308, 22250738585072014, -308),
        (1.7976931348623157e308, 17976931348623157, 308),
        // 2^-25: exactly halfway between two 17-digit candidates; the even
        // last digit wins.
        (2.9802322387695312e-08, 29802322387695312, -8),
        (9007199254740994.0, 9007199254740994, 15),
        (2e23, 2, 23),
        (123456789012345680.0, 12345678901234568, 17),
    ];

    #[test]
    fn digits_match_host_python() {
        for &(x, d, e10) in TABLE {
            assert_eq!(shortest_digits(x), (d, e10), "{x:e}");
        }
    }

    #[test]
    fn exact_ties_round_to_even_like_python() {
        // Rust's own `{:e}` rounds these ties up; python3 and node print the
        // even neighbour.
        assert_eq!(format!("{:e}", 2f64.powi(-25)), "2.9802322387695313e-8");
        assert_eq!(shortest_digits(2f64.powi(-25)), (29802322387695312, -8));
        // 1072012167405248.25 exactly (the ulp here is 1/8), a tie between
        // …248.2 and …248.3 at 17 digits (Python and JavaScript print …248.2).
        let tie = 1_072_012_167_405_248.0 + 0.25;
        assert_eq!(shortest_digits(tie), (10720121674052482, 15));
        // Python prints 5.960464477539062e-07.
        assert_eq!(shortest_digits(5.960464477539063e-7), (5960464477539062, -7));
        assert_eq!(increment_digits("1299", 3), ("1300".to_string(), 3));
        assert_eq!(increment_digits("999", 3), ("1".to_string(), 4));
    }

    /// The integer digits agree with the core's digit string, and the three
    /// ops give 0 for the classes without digits.
    #[test]
    fn neutral_primitives_follow_the_core() {
        for &(x, ..) in TABLE {
            let (d, e10) = shortest_digits(x);
            let (s, se10) = shortest_digit_string(x);
            assert_eq!((d.to_string(), e10), (s, se10), "{x:e}");
            assert_eq!(f64_class(x), 1);
            assert_eq!(f64_digits(x), d as i64);
            assert_eq!(f64_exp10(x), i64::from(e10));
            assert!(d % 10 != 0 && d < 100_000_000_000_000_000, "{x:e}: D = {d}");
        }
        assert_eq!(shortest_digits(1.5), (15, 0));
        assert_eq!(shortest_digits(1e21), (1, 21));
        for (x, class) in [(0.0, 0), (-0.0, 0), (f64::INFINITY, 2), (f64::NEG_INFINITY, 2), (f64::NAN, 3), (-f64::NAN, 3)] {
            assert_eq!((f64_class(x), f64_digits(x), f64_exp10(x)), (class, 0, 0), "{x:e}");
        }
        assert_eq!(f64_class(-5e-324), 1, "subnormals are class 1");
    }

    /// Every `(D, e10)` reads back as the same double, and is the shortest
    /// such: one digit fewer (rounded either way) never reads back.
    #[test]
    fn shortest_digits_round_trip() {
        let mut state = 0x9E37_79B9_7F4A_7C15_u64;
        for _ in 0..20_000 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let x = f64::from_bits(state);
            if !x.is_finite() || x == 0.0 {
                continue;
            }
            let (d, e10) = shortest_digits(x);
            let n = d.to_string().len() as i32;
            let back: f64 = format!("{d}e{}", e10 - n + 1).parse().unwrap();
            assert_eq!(back.to_bits(), x.abs().to_bits(), "{d}e{} does not read back as {x:e}", e10 - n + 1);
            if n > 1 {
                for shorter in [d / 10, d / 10 + 1] {
                    let y: f64 = format!("{shorter}e{}", e10 - n + 2).parse().unwrap();
                    assert_ne!(y.to_bits(), x.abs().to_bits(), "{x:e}: {shorter} is shorter than {d}");
                }
            }
        }
    }
}
