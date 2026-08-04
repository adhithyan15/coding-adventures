//! The Scilab value model.
//!
//! Scilab's numeric universe is the array, exactly like MATLAB's — the
//! workhorse value is an [`array_runtime::Array`] (a dense, column-major
//! `f64` matrix; a scalar is `1×1`). Strings (`'abc'`/`"abc"` — the same
//! underlying type in Scilab, MA10 §3) are a thin string layer on top.
//! Logicals are ordinary `0.0`/`1.0` numeric arrays, matching this repo's own
//! MATLAB/APL/J convention (`%t`/`%f` in `builtins.rs` produce exactly these).
//!
//! ## Why `ScilabValue` is its own enum, not `matlab_runtime::MatValue`
//!
//! `code/specs/MA10-scilab-language.md` §2 is explicit that this is the
//! decisive design choice of the whole language: reusing `MatValue` would
//! silently reuse MATLAB's answer to "what does an operator mean on this
//! variant" — and MA10 §1 finding 1 is that `+` means two genuinely different
//! things on strings in the two languages (MATLAB: ASCII-code numeric
//! addition; Scilab: concatenation). `ScilabValue` has the identical *shape*
//! to `MatValue` (`Num(Array)` + a string wrapper) but is a completely
//! separate type with no operator implementation over its `Str` variant at
//! all (see [`ScilabValue::as_num`] below) — so there is no code path,
//! anywhere in this crate, that could accidentally compute a MATLAB-shaped
//! answer for `'a' + 'b'` by falling through to a shared implementation.
//! MA10 §4 scopes strings to assignment, display, and equality only; even
//! implementing `+` as Scilab's *own* (correct, concatenation) answer is
//! explicitly deferred, because doing so honestly requires a typed-dispatch
//! layer MA10 §2 does not build in this cut — see `eval.rs`'s own
//! `apply_binop` for where that boundary is enforced at the operator-dispatch
//! level.

use array_runtime::Array;
use std::fmt;

/// A Scilab value: a numeric (or logical) array, or a string.
#[derive(Clone, Debug)]
pub enum ScilabValue {
    /// A numeric (or logical) array — the common case, and the *only* case
    /// any arithmetic/comparison-ordering/logical operator ever touches.
    Num(Array),
    /// A string (`'...'` or `'"..."'` — the same type, MA10 §3). Assignment,
    /// display, and `==`/`~=`/`<>` equality are the entire surface this cut
    /// gives it (MA10 §4) — no arithmetic, no concatenation, no ordering.
    Str(String),
}

impl ScilabValue {
    /// A `1×1` numeric scalar.
    pub fn scalar(x: f64) -> ScilabValue {
        ScilabValue::Num(Array::scalar(x))
    }

    /// Borrow the underlying numeric array, or error with `ctx` if this is a
    /// string. This is the ONE gate every arithmetic/matrix operator and
    /// every ordering comparison (`< <= > >=`) goes through — there is no
    /// sibling `as_str`-plus-dispatch that lets a string reach any of them
    /// (MA10 §4's scope cut: no operator, especially `+`, over strings).
    /// `==`/`~=`/`<>` are handled *before* this gate is ever reached (see
    /// `eval.rs::apply_binop`'s own doc comment), since those three ARE
    /// defined for two strings — string equality never calls this method.
    pub fn as_num(&self, ctx: &str) -> Result<&Array, String> {
        match self {
            ScilabValue::Num(a) => Ok(a),
            ScilabValue::Str(_) => {
                Err(format!("{ctx}: operation is not defined for strings"))
            }
        }
    }

    /// Scilab truthiness for `if`/`while`/`select`-implied conditions: a
    /// numeric array is true iff it is non-empty and *every* element is
    /// non-zero (identical rule to MATLAB's own `MatValue::is_true` —
    /// nothing in MA10 documents Scilab diverging from this convention, and
    /// `%t`/`%f` being ordinary `1.0`/`0.0` scalars, MA10 §4, means this
    /// numeric rule already covers boolean conditions for free). A non-empty
    /// string is true, matching `MatValue::Char`'s identical treatment.
    pub fn is_true(&self) -> bool {
        match self {
            ScilabValue::Num(a) => !a.is_empty() && a.data().iter().all(|&x| x != 0.0),
            ScilabValue::Str(s) => !s.is_empty(),
        }
    }
}

impl fmt::Display for ScilabValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScilabValue::Num(a) => write!(f, "{a}"),
            ScilabValue::Str(s) => write!(f, "{s}"),
        }
    }
}

/// Render a value the way an interactive Scilab session echoes an
/// unsuppressed result: `name = <value>`. A scalar prints on the same line; a
/// matrix on the lines below.
///
/// MA10 does not document Scilab's own echo format anywhere in the spec (it
/// is silent on session-transcript formatting), so this follows MATLAB's own
/// `name = value` convention as the closest documented precedent — the same
/// judgment call `matlab_runtime::echo` already made verbatim, applied here
/// for consistency across the two sibling frontends. Flagged for a reviewer
/// as a judgment call, not a spec-mandated format.
pub fn echo(name: &str, value: &ScilabValue) -> String {
    match value {
        ScilabValue::Num(a) if a.is_scalar() => format!("{name} = {a}"),
        ScilabValue::Str(s) => format!("{name} = {s}"),
        ScilabValue::Num(a) => format!("{name} =\n\n{a}\n"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_num_errors_cleanly_on_a_string() {
        let s = ScilabValue::Str("hi".to_string());
        assert!(s.as_num("test").is_err());
    }

    #[test]
    fn as_num_succeeds_on_a_number() {
        let n = ScilabValue::scalar(3.0);
        assert!(n.as_num("test").is_ok());
    }

    #[test]
    fn truthiness_matches_matlab_convention() {
        assert!(ScilabValue::scalar(1.0).is_true());
        assert!(!ScilabValue::scalar(0.0).is_true());
        assert!(ScilabValue::Str("x".to_string()).is_true());
        assert!(!ScilabValue::Str(String::new()).is_true());
    }

    #[test]
    fn echo_formats_scalars_and_strings_on_one_line() {
        assert_eq!(echo("x", &ScilabValue::scalar(5.0)), "x = 5");
        assert_eq!(
            echo("s", &ScilabValue::Str("hi".to_string())),
            "s = hi"
        );
    }
}
