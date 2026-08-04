//! Q-style display formatting.
//!
//! `array_runtime::value::Array` already implements [`std::fmt::Display`],
//! but (exactly like `apl-runtime::value::display`/`j-runtime::value::display`,
//! whose rationale this module mirrors) that impl is tuned for MATLAB, not
//! for a Q console session. Q's own conventions differ from `Array`'s
//! default `Display` in the same places APL's/J's did:
//!
//! | Rule                | MATLAB (`Array::fmt`)         | Q (this module)              |
//! |----------------------|--------------------------------|--------------------------------|
//! | Negative sign         | ASCII `-3`                    | ASCII `-3` (same!)            |
//! | Name prefix           | caller adds `x = `            | none — bare value              |
//! | Whole-valued float     | `3` (no `.0`)                 | `3` (no `.0`)                  |
//! | Vector layout          | n/a (row vector = 1×n matrix) | one line, space-separated      |
//! | Matrix layout           | 2-space gutter, right-aligned | 1-space gutter, right-aligned |
//!
//! ## Why a plain ASCII `-`, not J's leading underscore
//!
//! J needed a leading underscore (`_3`) for its negative-literal spelling
//! because J's own lexer reserves ASCII `-` exclusively for the `MINUS` verb
//! token (MA06 §4) — printing `-3` in J would be genuinely ambiguous if
//! pasted back into a session. Q is different (MA11 §3 bullet 2): Q spells
//! a negative *literal* with an ordinary leading `-`, disambiguated from
//! subtraction purely by *whitespace* (`2 -1` strands to `[2, -1]`; `2 - 1`
//! and `2-1` both subtract). This module's plain-`-` output round-trips
//! correctly through that exact rule: printing a vector as space-separated
//! elements (`"2 -1"`) re-tokenizes via `q-lexer`'s
//! `fold_negative_number_literals` hook back into the same two-element
//! strand, since there is a space before the `-` and none after. Using J's
//! underscore convention here would be actively *wrong* for Q — it is not a
//! free stylistic choice, it follows directly from Q's own real negative-
//! number spelling (MA11 §3 bullet 2's own callout that this is "not a free
//! choice ... MA11 §3 bullet 2 documents it as Q's own actual, real
//! disambiguation rule").
use crate::eval::QValue;
use array_runtime::Array;

/// Render a [`QValue`] the way a Q session echoes an unsuppressed result:
/// no name prefix, just the bare value.
///
/// A function value (`QValue::Fn`) has no literal source text kept around
/// to reproduce verbatim (this crate stores a parsed parameter list + body
/// AST, not the original source span) — printing one at the REPL is a rare,
/// cosmetic edge case with no behavioral significance (MA11 §4 never tests
/// or requires reconstructing a lambda's source), so this prints a small,
/// honestly-labelled placeholder (`{[x;y] ...}`) rather than attempting a
/// full, unparsing-based source reconstruction that the spec never asked
/// for.
pub fn display(v: &QValue) -> String {
    match v {
        QValue::Arr(a) => display_array(a),
        QValue::Fn(lambda) => format!("{{[{}] ...}}", lambda.params.join(";")),
    }
}

/// Render `a` the way a Q session echoes an unsuppressed array result.
///
/// - A scalar (`shape == []`) prints as its one number.
/// - A vector (`shape == [n]`) prints as its elements, space-separated, on
///   one line (the empty vector prints as the empty string).
/// - A matrix (`shape == [r, c]`) prints one row per line, elements
///   space-separated and right-aligned to the widest cell's width *in this
///   display*.
pub fn display_array(a: &Array) -> String {
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
            let cells: Vec<String> = a.data().iter().map(|&x| fmt_num(x)).collect();
            let width = cells.iter().map(|s| s.chars().count()).max().unwrap_or(1);
            let mut lines = Vec::with_capacity(r);
            for row in 0..r {
                let row_cells: Vec<String> = (0..c)
                    .map(|col| {
                        // `Array` stores data column-major, but `display_array`
                        // walks row-major -- one printed line per logical row
                        // (mirrors `j-runtime::value::display` exactly).
                        let x = a.get(row, col).expect("row/col in bounds");
                        format!("{:>width$}", fmt_num(x), width = width)
                    })
                    .collect();
                lines.push(row_cells.join(" "));
            }
            lines.join("\n")
        }
        // This crate (like `array-runtime` itself) is scoped to rank <= 2 --
        // every value this evaluator ever produces stops there, so this arm
        // is unreachable in practice. It exists only so `display_array` is
        // total over `Array`'s public shape space rather than panicking if
        // that ever changes.
        _ => format!("{a}"),
    }
}

/// Format one number the Q way: an ordinary ASCII `-` for negatives (see
/// this module's own doc comment for why this differs from J's leading
/// underscore), and a whole-valued float prints without a trailing `.0`
/// (`5`, not `5.0`).
///
/// Non-finite values (`inf`/`NaN`, reachable via monadic `%` on `0`, or a
/// `0%0`-shaped computation) still get a readable rendering, mirroring
/// `apl-runtime`'s/`j-runtime`'s own choice not to special-case these away.
fn fmt_num(x: f64) -> String {
    if x.is_nan() {
        return "NaN".to_string();
    }
    if x.is_infinite() {
        return if x.is_sign_negative() {
            "-inf".to_string()
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
    // `-0.0` is sign-negative but has zero magnitude -- there is no notion
    // of a "negative zero" literal in this cut, so treat it as plain `0`.
    if x.is_sign_negative() && mag != 0.0 {
        format!("-{body}")
    } else {
        body
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_displays_bare() {
        assert_eq!(display_array(&Array::scalar(3.0)), "3");
        assert_eq!(display_array(&Array::scalar(3.5)), "3.5");
    }

    #[test]
    fn negative_numbers_use_plain_ascii_minus() {
        // Unlike J's leading underscore, Q spells a negative literal with
        // an ordinary `-` (MA11 §3 bullet 2) -- this is the single most
        // important assertion in this module.
        assert_eq!(display_array(&Array::scalar(-3.0)), "-3");
        assert_eq!(display_array(&Array::scalar(-3.5)), "-3.5");
    }

    #[test]
    fn negative_zero_prints_as_plain_zero() {
        assert_eq!(display_array(&Array::scalar(-0.0)), "0");
    }

    #[test]
    fn whole_valued_floats_have_no_trailing_dot_zero() {
        assert_eq!(display_array(&Array::scalar(5.0)), "5");
        assert_eq!(display_array(&Array::scalar(5.25)), "5.25");
    }

    #[test]
    fn vector_is_space_separated_one_line_and_round_trips_negatives() {
        let v = Array::from_vec(vec![1.0, -2.0, 3.0]);
        let printed = display_array(&v);
        assert_eq!(printed, "1 -2 3");
        // Round-trip check: re-tokenizing the printed form must fold the
        // "-2" back into a single negative NUMBER token (space before `-`,
        // none after), not MINUS -- exactly the property this module's doc
        // comment claims.
        let tokens = coding_adventures_q_lexer::tokenize_q(&printed);
        let numbers: Vec<&str> = tokens
            .iter()
            .filter(|t| t.effective_type_name() == "NUMBER")
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(numbers, vec!["1", "-2", "3"]);
    }

    #[test]
    fn empty_vector_displays_as_empty_string() {
        assert_eq!(display_array(&Array::from_vec(vec![])), "");
    }

    #[test]
    fn matrix_is_row_per_line_right_aligned() {
        let m = Array::from_rows(vec![vec![1.0, 20.0], vec![300.0, 4.0]]).unwrap();
        assert_eq!(display_array(&m), "  1  20\n300   4");
    }

    #[test]
    fn matrix_with_negatives_uses_ascii_minus_in_alignment() {
        let m = Array::from_rows(vec![vec![1.0, -20.0], vec![-3.0, 4.0]]).unwrap();
        // Widest cell is "-20" (3 characters).
        assert_eq!(display_array(&m), "  1 -20\n -3   4");
    }

    #[test]
    fn infinities_and_nan_render_readably() {
        assert_eq!(display_array(&Array::scalar(f64::INFINITY)), "inf");
        assert_eq!(display_array(&Array::scalar(f64::NEG_INFINITY)), "-inf");
        assert_eq!(display_array(&Array::scalar(f64::NAN)), "NaN");
    }
}
