//! APL-style display formatting for `array_runtime::Array`.
//!
//! `array_runtime::value::Array` already implements [`std::fmt::Display`],
//! but that impl is tuned for MATLAB (ASCII `-` for negatives, a `name = `
//! echo owned by the *caller*, not the value itself). APL's console
//! conventions are different in exactly the ways a textbook session cares
//! about, so this module renders values the historical APL\360 way instead
//! of reusing `Array`'s own `Display`:
//!
//! | Rule                          | MATLAB (`Array::fmt`) | APL (this module) |
//! |--------------------------------|------------------------|--------------------|
//! | Negative sign                  | ASCII `-3`             | high-minus `¯3`    |
//! | Name prefix                    | caller adds `x = `     | none — bare value  |
//! | Whole-valued float              | `3` (no `.0`)          | `3` (no `.0`)      |
//! | Vector layout                  | n/a (row vector = 1×n matrix) | one line, space-separated |
//! | Matrix layout                  | 2-space gutter, right-aligned | 1-space gutter, right-aligned |
//!
//! The high-minus glyph `¯` (U+00AF, MACRON) is the exact character
//! `apl.tokens`' `NUMBER` rule already uses for negative literals (see
//! `code/grammars/apl/apl.tokens` SECTION 4) — using it here for *output*
//! keeps input and output symmetric: a negative number an APL session prints
//! is valid APL source if pasted back in.

use array_runtime::Array;

/// Render `a` the way an APL session echoes an unsuppressed result: no name
/// prefix, no `ans =` — just the bare value.
///
/// - A scalar (`shape == []`) prints as its one number.
/// - A vector (`shape == [n]`) prints as its elements, space-separated, on
///   one line (the empty vector prints as the empty string — an APL session
///   shows a blank line for `⍳0`, `⍴5`, etc.).
/// - A matrix (`shape == [r, c]`) prints one row per line, elements
///   space-separated and right-aligned to the widest cell's width *in this
///   display* (matching `Array`'s own matrix alignment convention, just with
///   APL's number formatting instead of MATLAB's).
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
                        // every APL session prints a matrix.
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

/// Format one number the APL way: [`is_sign_negative`](f64::is_sign_negative)
/// numbers get the high-minus `¯` prefix (never ASCII `-`), and a
/// whole-valued float prints without a trailing `.0` (`5`, not `5.0`).
/// Non-finite values (`inf`/`NaN`, reachable via monadic `÷` on `0` or a
/// `0÷0`-shaped computation — this crate deliberately does not special-case
/// division by zero, see `eval.rs`) still get a readable rendering rather
/// than Rust's default `inf`/`NaN` spelling colliding with APL's own accent
/// conventions.
fn fmt_num(x: f64) -> String {
    if x.is_nan() {
        return "NaN".to_string();
    }
    if x.is_infinite() {
        return if x.is_sign_negative() {
            "¯∞".to_string()
        } else {
            "∞".to_string()
        };
    }
    let mag = x.abs();
    let body = if mag.fract() == 0.0 && mag < 1e15 {
        format!("{}", mag as i64)
    } else {
        format!("{mag}")
    };
    // `-0.0` is sign-negative but has zero magnitude; APL has no notion of a
    // "negative zero" glyph, so treat it as plain `0`.
    if x.is_sign_negative() && mag != 0.0 {
        format!("¯{body}")
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
    fn negative_numbers_use_high_minus_not_ascii() {
        assert_eq!(display(&Array::scalar(-3.0)), "¯3");
        assert_eq!(display(&Array::scalar(-3.5)), "¯3.5");
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
        assert_eq!(display(&v), "1 ¯2 3");
    }

    #[test]
    fn empty_vector_displays_as_empty_string() {
        assert_eq!(display(&Array::from_vec(vec![])), "");
    }

    #[test]
    fn matrix_is_row_per_line_right_aligned() {
        // [[1, 20], [300, 4]] -- right-aligned to the widest cell ("300"),
        // cells separated by a single space (unlike `Array`'s own 2-space
        // gutter, see this module's doc comment table).
        let m = Array::from_rows(vec![vec![1.0, 20.0], vec![300.0, 4.0]]).unwrap();
        assert_eq!(display(&m), "  1  20\n300   4");
    }

    #[test]
    fn matrix_with_negatives_uses_high_minus_in_alignment() {
        let m = Array::from_rows(vec![vec![1.0, -20.0], vec![-3.0, 4.0]]).unwrap();
        // Widest cell is "¯20" (3 characters, counting ¯ as one character).
        assert_eq!(display(&m), "  1 ¯20\n ¯3   4");
    }

    #[test]
    fn infinities_and_nan_render_readably() {
        assert_eq!(display(&Array::scalar(f64::INFINITY)), "∞");
        assert_eq!(display(&Array::scalar(f64::NEG_INFINITY)), "¯∞");
        assert_eq!(display(&Array::scalar(f64::NAN)), "NaN");
    }
}
