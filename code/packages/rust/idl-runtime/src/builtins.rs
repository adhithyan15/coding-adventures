//! IDL's built-in routine surface -- a small, **explicitly scoped** cut,
//! per this task's own instruction ("document exactly which builtins you
//! included and why that set, not a vague 'some builtins'").
//!
//! ## Exactly what is included, and why
//!
//! - **`PRINT`** (procedure) -- MA-12d's own brief names this "the
//!   fundamental output primitive"; nothing in this cut is observable
//!   without it.
//! - **Trig/basic math** (functions, monadic, elementwise): `SIN` `COS`
//!   `TAN` `SQRT` `ABS` `EXP` `ALOG` (natural log -- IDL's real name for
//!   `ln`) `ALOG10` (base-10 log). A deliberately small "textbook session"
//!   set -- enough to write and check ordinary numeric programs -- not the
//!   much larger real IDL math-function library (MA12 §4's own "the wider
//!   intrinsic library... is deferred" scope boundary).
//! - **Array construction** (functions): the `*INDGEN` index-filled family
//!   (`INDGEN` `FINDGEN` `DINDGEN` `LINDGEN`) and the `*ARR` zero-filled
//!   family (`INTARR` `FLTARR` `DBLARR` `LONARR`) MA12 §4 names explicitly.
//!   All four members of each family are **identical** in this cut (MA12
//!   §2/§4's own deferred-typed-numeric-tower note: every value is `f64`
//!   regardless of which spelling constructed it) -- included as four
//!   separate names anyway, for source compatibility with real IDL
//!   programs that use any of the four spellings.
//! - **Reductions** (functions): `TOTAL` `MIN` `MAX` `N_ELEMENTS` `SIZE`,
//!   MA12 §4's own named "small, idiomatic set." `SIZE` implements exactly
//!   four modes: the default (no keyword) classic IDL dimension vector
//!   `[N_DIM, dim1, ..., dimN, TYPE, N_ELEMENTS]`, `/N_DIMENSIONS` (cited
//!   directly by MA12 §2), `/DIMENSIONS`, and `/N_ELEMENTS` -- every other
//!   real `SIZE` keyword mode (`/TYPE`, `/TNAME`, `/STRUCTURE`,
//!   `/FILE_LUN`, ...) is out of scope. `TYPE` is always reported as `5`
//!   (`DOUBLE`) for a numeric value and `7` (`STRING`) for a string --
//!   real IDL type codes, but this cut has only one numeric representation
//!   (the typed tower is deferred), so every number reports the same code.
//! - **`TRANSPOSE`** (function) -- MA12 §4's own explicit "transpose
//!   `TRANSPOSE(a)`" bullet.
//!
//! `#`/`##` (matrix product) and `^` (power) are ordinary *operators*, not
//! named routines -- implemented directly in `eval.rs`'s expression
//! cascade, not here.
//!
//! Everything else in real IDL's intrinsic library (`PLOT` and all
//! graphics, string functions, file I/O, ...) is out of scope, matching
//! MA12 §4's own deferred-surface list.

use crate::value::IdlValue;
use array_runtime::{ops, Array};
use std::collections::HashMap;

/// Upper bound on any array this crate allocates from a **runtime-computed**
/// size (an `*ARR`/`*INDGEN` length, a subscript range's element count, an
/// array literal's combined length) -- checked *before* allocating, the
/// same "check before the expensive work" discipline
/// `q_runtime::builtins::MAX_ARRAY_LENGTH` already established for this
/// repo's array-family runtimes.
pub const MAX_ARRAY_LENGTH: usize = 1_000_000;

/// The number of elements `v` reports for `N_ELEMENTS` -- a string is
/// always exactly one element (MA12 §2: a string is a *scalar*, no string
/// arrays this cut).
pub fn element_count(v: &IdlValue) -> usize {
    match v {
        IdlValue::Num(a) => a.len(),
        IdlValue::Str(_) => 1,
    }
}

/// Dispatch a built-in **procedure** call by (already-uppercased) name.
/// `None` means "not a recognized built-in name" -- the caller falls
/// through to the user-defined `procs` table. `Some(Ok(text))` is the line
/// of output text to emit (without its own trailing newline, added by the
/// caller's `emit`).
pub fn call_procedure(
    name: &str,
    positional: &[IdlValue],
    _keywords: &HashMap<String, IdlValue>,
) -> Option<Result<String, String>> {
    match name {
        // PRINT, a, b, c -- MA12 §4's fundamental output primitive. Real
        // IDL's own default (non-FORMAT) column layout/precision is not
        // independently pinned down byte-for-byte by the official docs
        // (they defer to the platform's own `sprintf`, confirmed directly
        // this session) -- this cut joins multiple arguments with a single
        // space on one line, a judgment call documented fully in
        // `value.rs`'s own `display` doc comment.
        "PRINT" => Some(Ok(positional
            .iter()
            .map(crate::value::display)
            .collect::<Vec<_>>()
            .join(" "))),
        _ => None,
    }
}

/// Dispatch a built-in **function** call by (already-uppercased) name.
/// `None` means "not a recognized built-in name" -- the caller falls
/// through to the user-defined `funcs` table.
pub fn call_function(
    name: &str,
    positional: &[IdlValue],
    keywords: &HashMap<String, IdlValue>,
) -> Option<Result<IdlValue, String>> {
    match name {
        "SIN" => Some(unary_math(name, positional, f64::sin)),
        "COS" => Some(unary_math(name, positional, f64::cos)),
        "TAN" => Some(unary_math(name, positional, f64::tan)),
        "SQRT" => Some(unary_math(name, positional, f64::sqrt)),
        "ABS" => Some(unary_math(name, positional, f64::abs)),
        "EXP" => Some(unary_math(name, positional, f64::exp)),
        "ALOG" => Some(unary_math(name, positional, f64::ln)),
        "ALOG10" => Some(unary_math(name, positional, f64::log10)),
        "INDGEN" | "FINDGEN" | "DINDGEN" | "LINDGEN" => Some(indgen(name, positional)),
        "INTARR" | "FLTARR" | "DBLARR" | "LONARR" => Some(arr_zeros(name, positional)),
        "TOTAL" => Some(reduce_one(name, positional, |a| {
            Ok(IdlValue::num(ops::sum(a)))
        })),
        "MIN" => Some(reduce_one(name, positional, |a| {
            Ok(IdlValue::num(ops::min(a)))
        })),
        "MAX" => Some(reduce_one(name, positional, |a| {
            Ok(IdlValue::num(ops::max(a)))
        })),
        "N_ELEMENTS" => Some(n_elements(positional)),
        "SIZE" => Some(size_fn(positional, keywords)),
        "TRANSPOSE" => Some(reduce_one(name, positional, |a| {
            Ok(IdlValue::Num(ops::transpose(a)))
        })),
        _ => None,
    }
}

fn require_one_num<'a>(name: &str, positional: &'a [IdlValue]) -> Result<&'a Array, String> {
    match positional {
        [IdlValue::Num(a)] => Ok(a),
        [IdlValue::Str(_)] => Err(format!(
            "idl-runtime: {name} does not accept a string argument"
        )),
        _ => Err(format!(
            "idl-runtime: {name} expects exactly 1 argument, got {}",
            positional.len()
        )),
    }
}

fn unary_math(
    name: &str,
    positional: &[IdlValue],
    f: impl Fn(f64) -> f64,
) -> Result<IdlValue, String> {
    let a = require_one_num(name, positional)?;
    let data: Vec<f64> = a.data().iter().map(|&x| f(x)).collect();
    Ok(IdlValue::Num(
        Array::from_shape(data, a.shape().to_vec()).expect("shape preserved"),
    ))
}

fn reduce_one(
    name: &str,
    positional: &[IdlValue],
    f: impl Fn(&Array) -> Result<IdlValue, String>,
) -> Result<IdlValue, String> {
    let a = require_one_num(name, positional)?;
    f(a)
}

fn n_elements(positional: &[IdlValue]) -> Result<IdlValue, String> {
    if positional.len() != 1 {
        return Err(format!(
            "idl-runtime: N_ELEMENTS expects exactly 1 argument, got {}",
            positional.len()
        ));
    }
    Ok(IdlValue::num(element_count(&positional[0]) as f64))
}

/// The `*INDGEN` family: each element set to its own 0-based subscript
/// (MA12 §4). Only the 1-D (single length argument) form is implemented in
/// this cut; a 2-D dimension-pair form is out of scope.
fn indgen(name: &str, positional: &[IdlValue]) -> Result<IdlValue, String> {
    let n = require_length_arg(name, positional)?;
    Ok(IdlValue::Num(Array::from_vec(
        (0..n).map(|i| i as f64).collect(),
    )))
}

/// The `*ARR` family: zero-filled. Supports both the 1-D form (`FLTARR(n)`)
/// and the 2-D form (`FLTARR(ncols, nrows)`, IDL's own `[column, row]`
/// dimension order per MA12 §2 -- `intarr(ncols, nrows)` "declares columns
/// first").
fn arr_zeros(name: &str, positional: &[IdlValue]) -> Result<IdlValue, String> {
    match positional.len() {
        1 => {
            let n = require_length_arg(name, positional)?;
            Ok(IdlValue::Num(Array::from_vec(vec![0.0; n])))
        }
        2 => {
            let ncols = require_dim(name, &positional[0])?;
            let nrows = require_dim(name, &positional[1])?;
            let total = ncols
                .checked_mul(nrows)
                .ok_or_else(|| format!("idl-runtime: {name} dimensions overflow"))?;
            if total > MAX_ARRAY_LENGTH {
                return Err(format!(
                    "idl-runtime: {name}({ncols}, {nrows}) exceeds the {MAX_ARRAY_LENGTH}-element cap"
                ));
            }
            Ok(IdlValue::Num(Array::zeros(nrows, ncols)))
        }
        n => Err(format!(
            "idl-runtime: {name} expects 1 or 2 arguments, got {n}"
        )),
    }
}

fn require_length_arg(name: &str, positional: &[IdlValue]) -> Result<usize, String> {
    if positional.len() != 1 {
        return Err(format!(
            "idl-runtime: {name} expects exactly 1 argument, got {}",
            positional.len()
        ));
    }
    require_dim(name, &positional[0])
}

fn require_dim(name: &str, v: &IdlValue) -> Result<usize, String> {
    let n = match v {
        IdlValue::Num(a) if a.len() == 1 => a.data()[0],
        _ => {
            return Err(format!(
                "idl-runtime: {name}'s dimension argument must be a scalar number"
            ))
        }
    };
    if n < 0.0 || n.fract() != 0.0 {
        return Err(format!(
            "idl-runtime: {name}'s dimension argument must be a non-negative integer, got {n}"
        ));
    }
    let n = n as usize;
    if n > MAX_ARRAY_LENGTH {
        return Err(format!(
            "idl-runtime: {name}({n}) exceeds the {MAX_ARRAY_LENGTH}-element cap"
        ));
    }
    Ok(n)
}

/// `(n_dims, dims, type_code, n_elements)` for `SIZE`'s own bookkeeping.
/// `type_code` mirrors real IDL's own codes (`5` = `DOUBLE`, `7` =
/// `STRING`) -- this cut's untyped `f64` model always reports `DOUBLE` for
/// a numeric value (the typed tower is deferred, MA12 §2).
fn dims_of(v: &IdlValue) -> (usize, Vec<usize>, i32, usize) {
    match v {
        IdlValue::Num(a) => (a.ndims(), a.shape().to_vec(), 5, a.len()),
        IdlValue::Str(_) => (0, Vec::new(), 7, 1),
    }
}

fn kw_truthy(keywords: &HashMap<String, IdlValue>, name: &str) -> bool {
    matches!(keywords.get(name), Some(IdlValue::Num(a)) if a.len() == 1 && a.data()[0] != 0.0)
}

/// `SIZE(x)`, `SIZE(x, /N_DIMENSIONS)`, `SIZE(x, /DIMENSIONS)`,
/// `SIZE(x, /N_ELEMENTS)` -- see this module's own top doc comment for
/// exactly which modes are (and are not) implemented.
fn size_fn(
    positional: &[IdlValue],
    keywords: &HashMap<String, IdlValue>,
) -> Result<IdlValue, String> {
    if positional.len() != 1 {
        return Err(format!(
            "idl-runtime: SIZE expects exactly 1 argument, got {}",
            positional.len()
        ));
    }
    let (ndims, dims, type_code, n_elements) = dims_of(&positional[0]);
    if kw_truthy(keywords, "N_DIMENSIONS") {
        return Ok(IdlValue::num(ndims as f64));
    }
    if kw_truthy(keywords, "N_ELEMENTS") {
        return Ok(IdlValue::num(n_elements as f64));
    }
    if kw_truthy(keywords, "DIMENSIONS") {
        return Ok(IdlValue::Num(Array::from_vec(
            dims.iter().map(|&d| d as f64).collect(),
        )));
    }
    let mut out = vec![ndims as f64];
    out.extend(dims.iter().map(|&d| d as f64));
    out.push(type_code as f64);
    out.push(n_elements as f64);
    Ok(IdlValue::Num(Array::from_vec(out)))
}

// ── Operators implemented here (shared by eval.rs's expression cascade) ───

/// Elementwise negation (unary `-`), preserving shape.
pub fn negate(a: &Array) -> Array {
    Array::from_shape(a.data().iter().map(|&x| -x).collect(), a.shape().to_vec())
        .expect("negate preserves shape/length")
}

/// Elementwise power (`^`), with the same scalar-broadcast-or-exact-shape
/// rule `array_runtime::ops::elementwise` uses. Not in `array_runtime::ops`
/// itself (no `BinOp::Pow` variant), so implemented here directly.
pub fn elementwise_pow(a: &Array, b: &Array) -> Result<Array, String> {
    broadcast(a, b, |x, y| x.powf(y), "^")
}

/// Elementwise BITWISE AND/OR/XOR/NOT (`AND`/`OR`/`XOR`/`NOT`) -- real IDL
/// documents these as bitwise operators over an integer representation,
/// NOT short-circuit boolean logic (confirmed directly against NV5
/// Geospatial's own *Bitwise Operators*/*Logical vs. Bitwise Operators*
/// pages this session; `&&`/`||`/`~`, the genuinely logical family, are
/// out of scope, MA12 §4). This cut truncates each `f64` operand to `i64`
/// before the bitwise op and converts the result back to `f64`.
pub fn bitwise_and(a: &Array, b: &Array) -> Result<Array, String> {
    broadcast(a, b, |x, y| ((x as i64) & (y as i64)) as f64, "AND")
}
pub fn bitwise_or(a: &Array, b: &Array) -> Result<Array, String> {
    broadcast(a, b, |x, y| ((x as i64) | (y as i64)) as f64, "OR")
}
pub fn bitwise_xor(a: &Array, b: &Array) -> Result<Array, String> {
    broadcast(a, b, |x, y| ((x as i64) ^ (y as i64)) as f64, "XOR")
}

/// `NOT x` -- a full bitwise complement (`!(x as i64)`), NOT a logical
/// negation. This is a well-known, documented real-IDL gotcha: `NOT 0` is
/// `-1` and `NOT 1` is `-2` -- **both nonzero**, so bitwise `NOT` cannot be
/// used to invert a 0/1 comparison's truthiness in an `IF` the way a
/// logical negation would. Faithfully reproduced here, not "fixed."
pub fn bitwise_not(a: &Array) -> Result<Array, String> {
    Ok(Array::from_shape(
        a.data().iter().map(|&x| !(x as i64) as f64).collect(),
        a.shape().to_vec(),
    )
    .expect("bitwise_not preserves shape/length"))
}

/// Shared scalar-broadcast-or-exact-shape elementwise helper, mirroring
/// `array_runtime::ops::elementwise`'s own broadcasting rule exactly (see
/// that function's own doc comment) for the operators `array-runtime`
/// itself has no `BinOp` variant for (`^`, and the bitwise family).
fn broadcast(
    a: &Array,
    b: &Array,
    f: impl Fn(f64, f64) -> f64,
    op_name: &str,
) -> Result<Array, String> {
    let (ad, bd) = (a.data(), b.data());
    let data: Vec<f64> = match (a.is_scalar(), b.is_scalar()) {
        (true, _) => bd.iter().map(|&y| f(ad[0], y)).collect(),
        (_, true) => ad.iter().map(|&x| f(x, bd[0])).collect(),
        _ => {
            if a.shape() != b.shape() {
                return Err(format!(
                    "idl-runtime: non-conformable arrays for {op_name}: {:?} vs {:?}",
                    a.shape(),
                    b.shape()
                ));
            }
            ad.iter().zip(bd).map(|(&x, &y)| f(x, y)).collect()
        }
    };
    let shape = if a.is_scalar() { b.shape() } else { a.shape() };
    Array::from_shape(data, shape.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_joins_positional_args_with_a_space() {
        let args = vec![IdlValue::num(1.0), IdlValue::Str("hi".to_string())];
        let result = call_procedure("PRINT", &args, &HashMap::new())
            .unwrap()
            .unwrap();
        assert_eq!(result, "1 hi");
    }

    #[test]
    fn unrecognized_procedure_name_returns_none() {
        assert!(call_procedure("NOT_A_REAL_PROC", &[], &HashMap::new()).is_none());
    }

    #[test]
    fn unrecognized_function_name_returns_none() {
        assert!(call_function("NOT_A_REAL_FUNC", &[], &HashMap::new()).is_none());
    }

    #[test]
    fn indgen_family_all_identical_in_this_cut() {
        for name in ["INDGEN", "FINDGEN", "DINDGEN", "LINDGEN"] {
            let result = call_function(name, &[IdlValue::num(3.0)], &HashMap::new())
                .unwrap()
                .unwrap();
            match result {
                IdlValue::Num(a) => assert_eq!(a.data(), &[0.0, 1.0, 2.0]),
                _ => panic!("expected Num"),
            }
        }
    }

    #[test]
    fn arr_family_2d_uses_column_row_order() {
        // FLTARR(ncols, nrows) -- MA12 §2's [column, row] dimension order.
        let result = call_function(
            "FLTARR",
            &[IdlValue::num(2.0), IdlValue::num(3.0)],
            &HashMap::new(),
        )
        .unwrap()
        .unwrap();
        match result {
            IdlValue::Num(a) => assert_eq!(a.shape(), &[3, 2]),
            _ => panic!("expected Num"),
        }
    }

    #[test]
    fn bitwise_not_matches_the_documented_idl_gotcha() {
        let zero = Array::scalar(0.0);
        let one = Array::scalar(1.0);
        assert_eq!(bitwise_not(&zero).unwrap().data(), &[-1.0]);
        assert_eq!(bitwise_not(&one).unwrap().data(), &[-2.0]);
    }

    #[test]
    fn bitwise_and_or_xor_of_01_values_match_logical() {
        let (zero, one) = (Array::scalar(0.0), Array::scalar(1.0));
        assert_eq!(bitwise_and(&one, &one).unwrap().data(), &[1.0]);
        assert_eq!(bitwise_and(&one, &zero).unwrap().data(), &[0.0]);
        assert_eq!(bitwise_or(&zero, &one).unwrap().data(), &[1.0]);
        assert_eq!(bitwise_xor(&one, &one).unwrap().data(), &[0.0]);
    }

    #[test]
    fn size_default_returns_the_classic_dimension_vector() {
        let v = IdlValue::Num(Array::from_vec(vec![1.0, 2.0, 3.0]));
        let result = call_function("SIZE", &[v], &HashMap::new())
            .unwrap()
            .unwrap();
        match result {
            // [N_DIM=1, dim1=3, TYPE=5 (double), N_ELEMENTS=3]
            IdlValue::Num(a) => assert_eq!(a.data(), &[1.0, 3.0, 5.0, 3.0]),
            _ => panic!("expected Num"),
        }
    }

    #[test]
    fn size_n_dimensions_keyword_matches_ma12_scalar_example() {
        // MA12 §2: "a scalar has zero dimensions."
        let scalar = IdlValue::num(42.0);
        let mut kw = HashMap::new();
        kw.insert("N_DIMENSIONS".to_string(), IdlValue::num(1.0));
        let result = call_function("SIZE", &[scalar], &kw).unwrap().unwrap();
        match result {
            IdlValue::Num(a) => assert_eq!(a.data(), &[0.0]),
            _ => panic!("expected Num"),
        }
    }

    #[test]
    fn n_elements_counts_a_string_as_one() {
        let s = IdlValue::Str("hello".to_string());
        let result = call_function("N_ELEMENTS", &[s], &HashMap::new())
            .unwrap()
            .unwrap();
        match result {
            IdlValue::Num(a) => assert_eq!(a.data(), &[1.0]),
            _ => panic!("expected Num"),
        }
    }

    #[test]
    fn dimension_arg_over_the_cap_is_rejected_before_allocating() {
        let huge = IdlValue::num((MAX_ARRAY_LENGTH + 1) as f64);
        assert!(call_function("INDGEN", &[huge], &HashMap::new())
            .unwrap()
            .is_err());
    }
}
