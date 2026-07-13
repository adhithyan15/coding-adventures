//! J-style display formatting for `array_runtime::Array`.
//!
//! `array_runtime::value::Array` already implements [`std::fmt::Display`],
//! but (exactly like `apl-runtime::value::display`, whose rationale this
//! module mirrors) that impl is tuned for MATLAB, not for a J console
//! session. J's own conventions differ from `Array`'s default `Display` in
//! the same places APL's did, plus one J-specific twist on the negative-sign
//! spelling:
//!
//! | Rule                     | MATLAB (`Array::fmt`)         | J (this module)             |
//! |---------------------------|--------------------------------|-------------------------------|
//! | Negative sign              | ASCII `-3`                    | leading underscore `_3`      |
//! | Name prefix                | caller adds `x = `            | none — bare value            |
//! | Whole-valued float          | `3` (no `.0`)                 | `3` (no `.0`)                 |
//! | Vector layout               | n/a (row vector = 1×n matrix) | one line, space-separated    |
//! | Matrix layout                | 2-space gutter, right-aligned | 1-space gutter, right-aligned|
//!
//! ## Why a leading underscore, not APL's high-minus `¯` or a bare ASCII `-`
//!
//! `j.tokens`' own `NUMBER` rule (SECTION 4) already settled this at the
//! *input* side: J has no Unicode high-minus glyph available (MA06 §4's
//! negative-literal addendum — J is ASCII-only, §1 bullet 1), so a negative
//! literal is spelled with a leading underscore (`_5`, `1.5E_3`) instead. A
//! bare ASCII `-` is unusable for this purpose because `j.tokens` already
//! reserves it exclusively for the `MINUS` verb token (subtraction/negate) —
//! printing `-3` here would be genuinely ambiguous if pasted back into a
//! session (is it the number `-3`, or `MINUS` followed by `3`?). Using the
//! *same* underscore convention for output that `j.tokens` already uses for
//! input keeps a printed session round-trippable, exactly the same
//! input/output symmetry `apl-runtime::value`'s own doc comment calls out
//! for APL's high-minus.
use array_runtime::Array;

/// Render `a` the way a J session echoes an unsuppressed result: no name
/// prefix — just the bare value.
///
/// - A scalar (`shape == []`) prints as its one number.
/// - A vector (`shape == [n]`) prints as its elements, space-separated, on
///   one line (the empty vector prints as the empty string — a J session
///   shows a blank line for `i.0`, `$5`, etc.).
/// - A matrix (`shape == [r, c]`) prints one row per line, elements
///   space-separated and right-aligned to the widest cell's width *in this
///   display*.
pub fn display(a: &Array) -> String {
    match *a.shape() {
        [] => fmt_num(a.data()[0]),
        [n] => {
            if n == 0 {
                return String::new();
            }
            a.data()
                .iter()
                .map(|&x| fmt_num(x))
                .collect::<Vec<_>>()
                .join(" ")
        }
        [r, c] => {
            // Format every cell once, up front, so the alignment width is
            // computed the same way regardless of row/column traversal order.
            let cells: Vec<String> = a.data().iter().map(|&x| fmt_num(x)).collect();
            let width = cells.iter().map(|s| s.chars().count()).max().unwrap_or(1);
            let mut lines = Vec::with_capacity(r);
            for row in 0..r {
                let row_cells: Vec<String> = (0..c)
                    .map(|col| {
                        // `Array` stores data column-major (`get` already
                        // hides that), but `display` walks row-major — one
                        // printed line per logical row — since that is how
                        // a J session prints a matrix (mirrors
                        // `apl-runtime::value::display` exactly).
                        let x = a.get(row, col).expect("row/col in bounds");
                        format!("{:>width$}", fmt_num(x), width = width)
                    })
                    .collect();
                lines.push(row_cells.join(" "));
            }
            lines.join("\n")
        }
        // This crate (like `array-runtime` itself, and `ops::reduce`/`scan`/
        // `outer`) is scoped to rank ≤ 2 — every value this evaluator ever
        // produces stops there, so this arm is unreachable in practice. It
        // exists only so `display` is total over `Array`'s public shape
        // space rather than panicking if that ever changes.
        _ => format!("{a}"),
    }
}

/// Format one number the J way: a
/// [`is_sign_negative`](f64::is_sign_negative) number gets a leading
/// underscore `_` prefix (never ASCII `-`, never APL's high-minus `¯` — see
/// this module's doc comment), and a whole-valued float prints without a
/// trailing `.0` (`5`, not `5.0`).
///
/// Non-finite values (`inf`/`NaN`, reachable via monadic `%` on `0`, a
/// `0%0`-shaped computation, or `^` overflowing — this crate deliberately
/// does not special-case any of these, mirroring
/// `apl-runtime::value::fmt_num`'s own choice not to) still get a readable
/// rendering. **Design judgment call, not specified by MA06**: this cut's
/// scope explicitly excludes J's own `_`/`__` infinity *literal* (MA06 §4's
/// negative-literal addendum), so `inf`/`_inf` here does not roundtrip back
/// into a parseable literal the way every finite number this module prints
/// does — that asymmetry is accepted deliberately (mirroring
/// `apl-runtime::value::fmt_num`'s own `¯∞`/`∞`, which has the identical
/// property: APL's `apl.tokens` has no infinity literal either), rather than
/// inventing a new literal syntax this cut's grammar doesn't have.
fn fmt_num(x: f64) -> String {
    if x.is_nan() {
        return "NaN".to_string();
    }
    if x.is_infinite() {
        return if x.is_sign_negative() {
            "_inf".to_string()
        } else {
            "inf".to_string()
        };
    }
    let mag = x.abs();
    let body = if mag.fract() == 0.0 && mag < 1e15 {
        format!("{}", mag as i64)
    } else {
        format!("{mag}")
    };
    // `-0.0` is sign-negative but has zero magnitude; there is no notion of
    // a "negative zero" literal in this cut, so treat it as plain `0`.
    if x.is_sign_negative() && mag != 0.0 {
        format!("_{body}")
    } else {
        body
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_displays_bare() {
        assert_eq!(display(&Array::scalar(3.0)), "3");
        assert_eq!(display(&Array::scalar(3.5)), "3.5");
    }

    #[test]
    fn negative_numbers_use_leading_underscore_not_ascii_minus() {
        assert_eq!(display(&Array::scalar(-3.0)), "_3");
        assert_eq!(display(&Array::scalar(-3.5)), "_3.5");
        assert!(!display(&Array::scalar(-3.0)).contains('-'));
    }

    #[test]
    fn negative_zero_prints_as_plain_zero() {
        assert_eq!(display(&Array::scalar(-0.0)), "0");
    }

    #[test]
    fn whole_valued_floats_have_no_trailing_dot_zero() {
        assert_eq!(display(&Array::scalar(5.0)), "5");
        assert_eq!(display(&Array::scalar(5.25)), "5.25");
    }

    #[test]
    fn vector_is_space_separated_one_line() {
        let v = Array::from_vec(vec![1.0, -2.0, 3.0]);
        assert_eq!(display(&v), "1 _2 3");
    }

    #[test]
    fn empty_vector_displays_as_empty_string() {
        assert_eq!(display(&Array::from_vec(vec![])), "");
    }

    #[test]
    fn matrix_is_row_per_line_right_aligned() {
        let m = Array::from_rows(vec![vec![1.0, 20.0], vec![300.0, 4.0]]).unwrap();
        assert_eq!(display(&m), "  1  20\n300   4");
    }

    #[test]
    fn matrix_with_negatives_uses_underscore_in_alignment() {
        let m = Array::from_rows(vec![vec![1.0, -20.0], vec![-3.0, 4.0]]).unwrap();
        // Widest cell is "_20" (3 characters).
        assert_eq!(display(&m), "  1 _20\n _3   4");
    }

    #[test]
    fn infinities_and_nan_render_readably() {
        assert_eq!(display(&Array::scalar(f64::INFINITY)), "inf");
        assert_eq!(display(&Array::scalar(f64::NEG_INFINITY)), "_inf");
        assert_eq!(display(&Array::scalar(f64::NAN)), "NaN");
    }
}
