//! Shortest round-trip float formatting in Python's and JavaScript's layouts
//! (LANG83 PR6a — `write_f64_py` / `write_f64_js`).
//!
//! # Two questions: which digits, and where the point goes
//!
//! Printing a float is two separate decisions.
//!
//! 1. **The digits.** Python's `repr(x)` and JavaScript's `String(x)` both
//!    print the *shortest* decimal digit string that reads back as exactly
//!    `x`. If several strings of that length read back as `x`, they print the
//!    one *closest* to `x`; on an exact tie, the one whose last digit is even.
//!    `0.1` is really `0.1000000000000000055511151231257827…`, but `0.1` is
//!    the shortest string that rounds back to it, so that is what you see.
//!
//! 2. **The layout.** Given digits `D` and the decimal exponent `e` of the
//!    first digit (`123.4` → `D = "1234"`, `e = 2`), the two languages differ:
//!
//!    | value | Python `repr` | JavaScript `toString` |
//!    |-------|---------------|-----------------------|
//!    | `1e16` | `1e+16` | `10000000000000000` |
//!    | `1e21` | `1e+21` | `1e+21` |
//!    | `1.5e-7` | `1.5e-07` | `1.5e-7` |
//!    | `100.0` | `100.0` | `100` |
//!    | `-0.0` | `-0.0` | `0` |
//!    | `inf` / `nan` | `inf` / `nan` | `Infinity` / `NaN` |
//!
//!    Python uses fixed notation when `-4 <= e < 16`; JavaScript when
//!    `-7 < e < 21`. Python pads the exponent to two digits; JavaScript does
//!    not. Python keeps `.0` on an integral value; JavaScript drops it.
//!
//! # Where the digits come from here
//!
//! Rust's `{:e}` formatting prints the shortest round-trip digits, closest to
//! the value — with ONE difference: when the value lies *exactly* halfway
//! between two shortest candidates, Rust takes the upper one, while Python,
//! JavaScript (ECMA-262 `Number::toString`, step 5's "choose the even one"),
//! Java 19+ and Ryu all take the one whose last digit is even. That is not
//! exotic: `2^-25` is exactly `2.98023223876953125e-08`, Python prints
//! `2.9802322387695312e-08` and Rust `2.9802322387695313e-8`; so does any
//! double near `1e15` with a `.25` or `.75` fraction (`1072012167405248.25`).
//! Measured against host `python3` on 535 432 values, Rust's digits differed
//! on 94 — every one of them such a tie.
//!
//! So this module reads Rust's shortest digits out of `format!("{:e}", x)`
//! (`"1.2345e-7"`, `"1e16"`), and then checks for that one case with Rust's
//! *exact* formatting (`{:.1100e}` prints every digit of the binary value —
//! a double has at most 767 significant decimal digits): if the exact
//! expansion is one digit longer than the shortest digits and ends in `5`, the
//! value is a tie, and the even neighbour is taken whenever it, too, reads back
//! as the same double. Both answers come from Rust's own formatting.
//!
//! The VM's default `write_f64_py` / `write_f64_js` builtins and the JIT's
//! embedder-registered handlers both call these functions, so the two
//! in-process columns cannot disagree.
//!
//! # The digits on their own (LANG83 PR7b)
//!
//! The layout is a language's rule; the digits are not. So the digits are
//! also exposed as three language-neutral integer queries — [`f64_class`],
//! [`f64_digits`] and [`f64_exp10`], the VM's (and the JIT's) lowering of the
//! registry ops of the same names — built on [`shortest_digits`], which
//! returns `(D, e10)` as integers. A compiler can then print a float in any
//! layout with integer arithmetic and `putchar`. Both paths share one core
//! (`shortest_digit_string`), so `format_f64`'s output is unchanged.

/// Which language's layout to use for the shortest digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatLayout {
    /// Python `repr(float)` (also `str` and what `print` writes).
    Python,
    /// JavaScript `Number.prototype.toString()` (what `console.log` writes).
    JavaScript,
}

/// The shortest round-trip digits of a finite, non-zero `x`, as a digit
/// STRING: `(digits, e10)` with `|x| ≈ 0.d₁ × 10^(e10+1)`, i.e. `e10` is the
/// decimal exponent of the first digit. `digits` has no leading or trailing
/// zeros.
///
/// This is the one internal core. [`shortest_digits`] (the LANG83 PR7b
/// `f64_digits` / `f64_exp10` answer) reads it as an integer, and
/// [`format_f64`] (PR6a's `write_f64_py` / `write_f64_js`) lays the string
/// out, so the ops and the writers cannot disagree on a digit.
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

/// Lay out shortest digits `digits` with first-digit exponent `e10`, for a
/// value whose sign has already been written (the result has no sign).
///
/// This is the one place the two layouts are spelled out; every backend's own
/// layout code (C, LLVM IR, WASM, JVM and CIL helpers, BEAM) follows it.
fn layout_digits(digits: &str, e10: i32, layout: FloatLayout) -> String {
    let n = digits.len() as i32;
    let (lo, hi) = match layout {
        // Python: fixed when -4 <= e < 16.
        FloatLayout::Python => (-4, 16),
        // JavaScript: fixed when -7 < e < 21, i.e. -6 <= e < 21.
        FloatLayout::JavaScript => (-6, 21),
    };
    let mut out = String::with_capacity(32);
    if (lo..hi).contains(&e10) {
        if e10 < 0 {
            // 0.000ddd — (-e10 - 1) zeros between the point and the digits.
            out.push_str("0.");
            out.extend(std::iter::repeat_n('0', (-e10 - 1) as usize));
            out.push_str(digits);
        } else if n <= e10 + 1 {
            // Integral: the digits, then zeros up to the units place.
            out.push_str(digits);
            out.extend(std::iter::repeat_n('0', (e10 + 1 - n) as usize));
            if layout == FloatLayout::Python {
                out.push_str(".0");
            }
        } else {
            // The point falls inside the digits.
            let split = (e10 + 1) as usize;
            out.push_str(&digits[..split]);
            out.push('.');
            out.push_str(&digits[split..]);
        }
    } else {
        // d[.ddd]e±X — a point only when more digits follow.
        out.push_str(&digits[..1]);
        if n > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        out.push('e');
        out.push(if e10 < 0 { '-' } else { '+' });
        let magnitude = e10.unsigned_abs();
        if layout == FloatLayout::Python && magnitude < 10 {
            out.push('0');
        }
        out.push_str(&magnitude.to_string());
    }
    out
}

/// Format `x` in `layout`: the special values, then the sign, then the
/// shortest digits laid out.
///
/// ```
/// use vm_core::float_format::{format_f64, FloatLayout::{JavaScript, Python}};
/// assert_eq!(format_f64(1e16, Python), "1e+16");
/// assert_eq!(format_f64(1e16, JavaScript), "10000000000000000");
/// assert_eq!(format_f64(-0.0, Python), "-0.0");
/// assert_eq!(format_f64(-0.0, JavaScript), "0");
/// ```
pub fn format_f64(x: f64, layout: FloatLayout) -> String {
    let py = layout == FloatLayout::Python;
    if x.is_nan() {
        // Python prints `nan` for every NaN, whatever its sign bit.
        return if py { "nan" } else { "NaN" }.to_string();
    }
    let sign = if x.is_sign_negative() { "-" } else { "" };
    if x.is_infinite() {
        return format!("{sign}{}", if py { "inf" } else { "Infinity" });
    }
    if x == 0.0 {
        // `0.0 == -0.0`, so this arm takes both zeros. Python keeps the sign
        // (`-0.0`); JavaScript's `Number::toString` prints `0` for both.
        return if py { format!("{sign}0.0") } else { "0".to_string() };
    }
    let (digits, e10) = shortest_digit_string(x);
    format!("{sign}{}", layout_digits(&digits, e10, layout))
}

/// Python `repr(x)` — what `write_f64_py` writes.
pub fn python_repr(x: f64) -> String {
    format_f64(x, FloatLayout::Python)
}

/// JavaScript `String(x)` — what `write_f64_js` writes.
pub fn js_to_string(x: f64) -> String {
    format_f64(x, FloatLayout::JavaScript)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `(value, Python repr, JavaScript String)` — every row was produced by
    /// host `python3` and `node`.
    const TABLE: &[(f64, &str, &str)] = &[
        (1.0, "1.0", "1"),
        (100.0, "100.0", "100"),
        (0.1, "0.1", "0.1"),
        (123.456, "123.456", "123.456"),
        (1e15, "1000000000000000.0", "1000000000000000"),
        (1e16, "1e+16", "10000000000000000"),
        (1e20, "1e+20", "100000000000000000000"),
        (1e21, "1e+21", "1e+21"),
        (1e-4, "0.0001", "0.0001"),
        (1e-5, "1e-05", "0.00001"),
        (1e-6, "1e-06", "0.000001"),
        (1e-7, "1e-07", "1e-7"),
        (1.5e-7, "1.5e-07", "1.5e-7"),
        (-2.5, "-2.5", "-2.5"),
        (5e-324, "5e-324", "5e-324"),
        (2.2250738585072014e-308, "2.2250738585072014e-308", "2.2250738585072014e-308"),
        (1.7976931348623157e308, "1.7976931348623157e+308", "1.7976931348623157e+308"),
        // 2^-25: exactly halfway between two 17-digit candidates; the even
        // last digit wins.
        (2.9802322387695312e-08, "2.9802322387695312e-08", "2.9802322387695312e-8"),
        (9007199254740994.0, "9007199254740994.0", "9007199254740994"),
        (2e23, "2e+23", "2e+23"),
        (123456789012345680.0, "1.2345678901234568e+17", "123456789012345680"),
    ];

    #[test]
    fn layouts_match_host_python_and_node() {
        for &(x, py, js) in TABLE {
            assert_eq!(python_repr(x), py, "python repr of {x:e}");
            assert_eq!(js_to_string(x), js, "js toString of {x:e}");
        }
    }

    #[test]
    fn exact_ties_round_to_even_like_python() {
        // Rust's own `{:e}` rounds these ties up; python3 and node print the
        // even neighbour.
        assert_eq!(format!("{:e}", 2f64.powi(-25)), "2.9802322387695313e-8");
        assert_eq!(python_repr(2f64.powi(-25)), "2.9802322387695312e-08");
        // 1072012167405248.25 exactly (the ulp here is 1/8), a tie between
        // …248.2 and …248.3 at 17 digits.
        let tie = 1_072_012_167_405_248.0 + 0.25;
        assert_eq!(python_repr(tie), "1072012167405248.2");
        assert_eq!(js_to_string(tie), "1072012167405248.2");
        assert_eq!(python_repr(5.960464477539063e-7), "5.960464477539062e-07");
        assert_eq!(increment_digits("1299", 3), ("1300".to_string(), 3));
        assert_eq!(increment_digits("999", 3), ("1".to_string(), 4));
    }

    #[test]
    fn special_values() {
        assert_eq!(python_repr(0.0), "0.0");
        assert_eq!(python_repr(-0.0), "-0.0");
        assert_eq!(js_to_string(0.0), "0");
        assert_eq!(js_to_string(-0.0), "0");
        assert_eq!(python_repr(f64::INFINITY), "inf");
        assert_eq!(python_repr(f64::NEG_INFINITY), "-inf");
        assert_eq!(js_to_string(f64::INFINITY), "Infinity");
        assert_eq!(js_to_string(f64::NEG_INFINITY), "-Infinity");
        assert_eq!(python_repr(f64::NAN), "nan");
        assert_eq!(python_repr(-f64::NAN), "nan");
        assert_eq!(js_to_string(f64::NAN), "NaN");
    }

    /// PR7b: the integer digits agree with the string the writers lay out,
    /// and the three ops give 0 for the classes without digits.
    #[test]
    fn neutral_primitives_match_the_layouts() {
        for &(x, py, _) in TABLE {
            let (d, e10) = shortest_digits(x);
            let (s, se10) = shortest_digit_string(x);
            assert_eq!((d.to_string(), e10), (s, se10), "{x:e}");
            assert_eq!(f64_class(x), 1);
            assert_eq!(f64_digits(x), d as i64);
            assert_eq!(f64_exp10(x), i64::from(e10));
            assert!(d % 10 != 0 && d < 100_000_000_000_000_000, "{x:e}: D = {d}");
            // The digits, re-read with the exponent, give the same double.
            let n = d.to_string().len() as i32;
            let back: f64 = format!("{d}e{}", e10 - n + 1).parse().unwrap();
            assert_eq!(back.to_bits(), x.abs().to_bits(), "{x:e} vs Python {py}");
        }
        assert_eq!(shortest_digits(1.5), (15, 0));
        assert_eq!(shortest_digits(1e21), (1, 21));
        assert_eq!(shortest_digits(2f64.powi(-25)), (29802322387695312, -8), "ties to even");
        for (x, class) in [(0.0, 0), (-0.0, 0), (f64::INFINITY, 2), (f64::NEG_INFINITY, 2), (f64::NAN, 3), (-f64::NAN, 3)] {
            assert_eq!((f64_class(x), f64_digits(x), f64_exp10(x)), (class, 0, 0), "{x:e}");
        }
        assert_eq!(f64_class(-5e-324), 1, "subnormals are class 1");
    }

    #[test]
    fn shortest_digits_round_trip() {
        // Every printed string must read back as the same bits.
        let mut state = 0x9E37_79B9_7F4A_7C15_u64;
        for _ in 0..20_000 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let x = f64::from_bits(state);
            if !x.is_finite() {
                continue;
            }
            for s in [python_repr(x), js_to_string(x)] {
                let back: f64 = s.parse().unwrap();
                assert_eq!(back.to_bits(), x.to_bits(), "{s} does not read back as {x:e}");
            }
        }
    }
}
