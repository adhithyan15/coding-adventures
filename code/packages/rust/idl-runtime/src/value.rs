//! `IdlValue` — IDL's runtime value model, and IDL's display/`PRINT`
//! convention.
//!
//! Per `code/specs/MA12-idl-language.md` §2, IDL's numeric core fits
//! `array-runtime` unchanged (0-based, column-major, a genuine rank-0
//! scalar) -- the only addition is a runtime-level string scalar. Mirroring
//! `ScilabValue`'s own precedent (MA10 §2) exactly, this crate gets **its
//! own** small value enum rather than reusing another language's -- reusing
//! e.g. `MatValue` would silently import MATLAB's own answer to "what does
//! `+` mean on this variant."
//!
//! ```text
//! IdlValue
//! ├── Num(Array)   -- every number in this cut, incl. scalars (shape []),
//! │                    vectors, and (via 2-D subscripting/ARR builtins)
//! │                    matrices. array-runtime is pure f64 -- IDL's typed
//! │                    numeric tower (INT/LONG/FLOAT/DOUBLE/...) is
//! │                    deferred (MA12 §2/§4), so this cut does not
//! │                    distinguish them.
//! └── Str(String)  -- a single- or double-quoted string SCALAR (MA12 §2/§4):
//!                      assignment, PRINT, equality, and keyword/positional
//!                      argument values only -- no string operators, no
//!                      string arrays, this cut.
//! ```

use array_runtime::Array;
use std::fmt;

/// IDL's runtime value: a numeric array (any rank this cut produces) or a
/// string scalar. See this module's own doc comment for why this is a new,
/// dedicated enum rather than a reused one.
#[derive(Debug, Clone)]
pub enum IdlValue {
    Num(Array),
    Str(String),
}

impl IdlValue {
    /// A convenience scalar constructor -- `IdlValue::Num(Array::scalar(x))`.
    pub fn num(x: f64) -> IdlValue {
        IdlValue::Num(Array::scalar(x))
    }

    /// The short, capitalized type name IDL itself uses in error messages
    /// (`"array"`/`"string"`) -- not a faithful reproduction of real IDL's
    /// own `TYPE_NAME()` strings (`"FLOAT"`, `"STRING"`, ...; the typed
    /// numeric tower is deferred, MA12 §2), just a readable label for this
    /// crate's own error text.
    pub fn type_name(&self) -> &'static str {
        match self {
            IdlValue::Num(_) => "array",
            IdlValue::Str(_) => "string",
        }
    }
}

/// Render an [`IdlValue`] the way this cut's `PRINT`/Implied-Print echo a
/// value: no name prefix, just the bare value.
///
/// # The display convention this module adopts, and what was verified vs. flagged
///
/// Per this task's own instruction, IDL's actual `PRINT` conventions were
/// checked directly (NV5 Geospatial's *PRINT/PRINTF*, *Output of IDL
/// Variables*, and *Implied Print* reference pages) rather than copied from
/// a prior sibling's own convention (Q's ASCII-minus, J's leading
/// underscore, Derive's own choice were each independently verified against
/// that language, per this repo's own discipline).
///
/// **Verified directly against the official docs:**
/// - Assignment produces **no** output; only a bare, non-assignment
///   top-level statement (a variable reference, a literal, a function call)
///   auto-prints via IDL's own **Implied Print** feature -- and Implied
///   Print does **not** fire inside a routine body (`PRO`/`FUNCTION`), only
///   at the interactive top level (*Implied Print*, confirmed directly).
///   `eval.rs::Interpreter::run` implements exactly this split.
/// - A comma-separated `PRINT, a, b, c` prints each expression to standard
///   output (*PRINT/PRINTF*, confirmed directly) -- this module joins them
///   with a single space on one line, since the official docs do not
///   specify the exact separator/layout for the default (non-`FORMAT`) case
///   (see the flagged judgment call below).
/// - IDL's array subscript ranges are documented as **inclusive of both
///   endpoints** (*Array Subscript Ranges*, confirmed directly) -- not a
///   `PRINT`-formatting fact, but confirmed the same way, and load-bearing
///   for `eval.rs`'s own subscript-range evaluation.
///
/// **Flagged as a judgment call, not independently verified to the byte:**
/// the official *PRINT/PRINTF*/*Output of IDL Variables* pages explicitly
/// state that default (`FORMAT`-less) numeric formatting goes through the
/// platform's own `sprintf` and differs slightly by platform, and do not
/// themselves spell out the exact default field width / decimal-place count
/// (community sources describe a fixed-decimal, multi-space-gutter,
/// right-justified convention with platform-dependent precision -- e.g. a
/// `FLOAT` scalar commonly prints with a fixed number of decimal places
/// rather than this crate's own trimmed style). Separately, *Implied Print*
/// itself documents showing *more* precision than plain `PRINT` does for the
/// same value (`1.2345678` vs. `PRINT`'s `1.23457`) -- a genuine difference
/// this crate does **not** reproduce (see below). Rather than fabricate a
/// specific field-width/precision scheme this session could not confirm
/// byte-for-byte, this module adopts the same "clean numeric echo" style
/// this repo's other array-family runtimes already use (`q-runtime::value`,
/// itself checked against Q's own real console): plain ASCII `-` for
/// negatives (IDL has no leading-underscore/whitespace-sensitive negative
/// convention the way J/Q do -- there is no ambiguity to guard against, so
/// this is a low-risk simplification, not a guess at something contested),
/// a whole-valued float prints without a trailing `.0`, a vector prints
/// space-separated on one line, and a matrix prints one row per line,
/// right-aligned to the widest cell. **Both `PRINT` and Implied Print share
/// this one formatting path in this cut** -- real IDL's own documented
/// precision difference between the two is not reproduced, a deliberate
/// simplification flagged here rather than silently presented as verified.
pub fn display(v: &IdlValue) -> String {
    match v {
        IdlValue::Num(a) => display_array(a),
        IdlValue::Str(s) => s.clone(),
    }
}

/// Render `a` the way this cut's shared numeric-display path renders an
/// array result -- see [`display`]'s own doc comment for the full
/// verified-vs-judgment-call breakdown.
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
                        let x = a.get(row, col).expect("row/col in bounds");
                        format!("{:>width$}", fmt_num(x), width = width)
                    })
                    .collect();
                lines.push(row_cells.join(" "));
            }
            lines.join("\n")
        }
        // This cut's evaluator never produces rank > 2 (array-runtime itself
        // only defines rank <= 2 ops, MA00 §3) -- unreachable in practice,
        // kept only so this function stays total over Array's public shape
        // space.
        _ => format!("{a}"),
    }
}

/// A minimal `Debug`-style label for error messages that need to name a
/// value's shape without fully rendering it (e.g. "expected a scalar, got a
/// 1x3 array").
pub fn describe(v: &IdlValue) -> String {
    match v {
        IdlValue::Num(a) => format!("{} array (shape {:?})", v.type_name(), a.shape()),
        IdlValue::Str(_) => "string".to_string(),
    }
}

impl fmt::Display for IdlValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", display(self))
    }
}

/// Format one number the way this crate's shared display path does: an
/// ordinary ASCII `-` for negatives, and a whole-valued float prints without
/// a trailing `.0` (`5`, not `5.0`). See [`display`]'s own doc comment for
/// why this specific style was chosen and what remains an unverified
/// judgment call.
fn fmt_num(x: f64) -> String {
    if x.is_nan() {
        return "NaN".to_string();
    }
    if x.is_infinite() {
        return if x.is_sign_negative() {
            "-Inf".to_string()
        } else {
            "Inf".to_string()
        };
    }
    let mag = x.abs();
    let body = if mag.fract() == 0.0 && mag < 1e15 {
        format!("{}", mag as i64)
    } else {
        format!("{mag}")
    };
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
    fn vector_is_space_separated_one_line() {
        let v = Array::from_vec(vec![1.0, -2.0, 3.0]);
        assert_eq!(display_array(&v), "1 -2 3");
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
    fn infinities_and_nan_render_readably() {
        assert_eq!(display_array(&Array::scalar(f64::INFINITY)), "Inf");
        assert_eq!(display_array(&Array::scalar(f64::NEG_INFINITY)), "-Inf");
        assert_eq!(display_array(&Array::scalar(f64::NAN)), "NaN");
    }

    #[test]
    fn string_displays_bare_no_quotes() {
        assert_eq!(display(&IdlValue::Str("hello".to_string())), "hello");
    }

    #[test]
    fn idlvalue_num_convenience_constructor() {
        assert_eq!(display(&IdlValue::num(42.0)), "42");
    }

    #[test]
    fn describe_names_shape_and_string() {
        assert!(describe(&IdlValue::Num(Array::from_vec(vec![1.0, 2.0]))).contains("[2]"));
        assert_eq!(describe(&IdlValue::Str("x".to_string())), "string");
    }
}
