//! The eight `%`-prefixed special constants, and the core Scilab built-in
//! functions, over `array-runtime`.
//!
//! ## Special constants
//!
//! `scilab.tokens`/`scilab.grammar` (MA-10b/MA-10c) already lex/parse
//! `%pi %e %i %inf %nan %eps %t %f` as a single, *closed* `PERCENT_CONST`
//! primary whose token *value* is the literal spelling (`"%pi"`, etc.) — see
//! `scilab-lexer`'s own `PERCENT_CONST` regex and `scilab-parser`'s `primary`
//! production. [`percent_const`] is the lookup table from that spelling to
//! its [`ScilabValue`] (MA10 §3/§4).
//!
//! `%i` is deliberately **not** a real imaginary unit: `array_runtime::Array`
//! is a pure `f64` (real-only) matrix with no complex-number representation
//! anywhere in its public API (confirmed directly by reading
//! `array-runtime/src/value.rs`/`ops.rs` — there is no `Complex`/`re`/`im`
//! concept at all), and MA10 §4's deferred list names "complex numbers"
//! explicitly, right alongside sparse matrices and N-D arrays. Silently
//! substituting some real number for `%i` (`0.0`? `1.0`?) would be exactly
//! the kind of "land on a plausible-looking but wrong answer" this whole
//! spec's §1 finding 1 was written to warn against for `+` — so `%i` is an
//! honest, clean `Err` here instead, the same "absence, not a guessed
//! substitute" discipline MA10 §4 already applies to the Kronecker operators
//! and the legacy `@`/`**` spellings.
//!
//! ## Built-in functions
//!
//! The starter set MA10 §4's in-scope surface actually needs — array
//! constructors (`zeros`/`ones`/`eye`), shape queries
//! (`size`/`length`/`numel`), whole-array reductions
//! (`sum`/`mean`/`max`/`min`), element-wise math (`abs`/`sqrt`),
//! `transpose`, and `disp` — every one a pure `array-runtime`
//! constructor/reduction with no MATLAB-specific semantics baked in, so
//! (mirroring `matlab-runtime::builtins` almost verbatim, just retyped over
//! [`ScilabValue`]) it transfers directly, unchanged, per MA10 §5's "zero
//! substrate work" conclusion.

use crate::value::ScilabValue;
use array_runtime::{ops, Array};

/// Resolve one of the eight fixed `PERCENT_CONST` spellings to its value.
///
/// `%t`/`%f` are ordinary `1.0`/`0.0` numeric scalars — this repo's
/// established "logicals are ordinary 0/1 numeric arrays" convention
/// (`matlab_runtime::MatValue`'s own doc comment states the identical rule
/// for MATLAB), not a distinct boolean variant.
pub fn percent_const(spelling: &str) -> Result<ScilabValue, String> {
    match spelling {
        "%pi" => Ok(ScilabValue::scalar(std::f64::consts::PI)),
        "%e" => Ok(ScilabValue::scalar(std::f64::consts::E)),
        "%i" => Err(
            "%i: complex numbers are not supported in this cut (MA10 §4 defers them; \
             array-runtime has no complex-number representation)"
                .to_string(),
        ),
        "%inf" => Ok(ScilabValue::scalar(f64::INFINITY)),
        "%nan" => Ok(ScilabValue::scalar(f64::NAN)),
        "%eps" => Ok(ScilabValue::scalar(f64::EPSILON)),
        "%t" => Ok(ScilabValue::scalar(1.0)),
        "%f" => Ok(ScilabValue::scalar(0.0)),
        other => Err(format!(
            "scilab-runtime: unknown special constant '{other}'"
        )),
    }
}

/// Dispatch a builtin by name. Returns `Err` for an unknown name (which the
/// evaluator reports as an undefined function/variable).
pub fn call(name: &str, args: &[ScilabValue]) -> Result<ScilabValue, String> {
    match name {
        "zeros" => constructor(name, args, 0.0),
        "ones" => constructor(name, args, 1.0),
        "eye" => eye(args),
        "size" => size(args),
        "length" => length(args),
        "numel" => numel(args),
        "sum" => reduce(name, args, ops::sum),
        "mean" => reduce(name, args, ops::mean),
        "max" => reduce(name, args, ops::max),
        "min" => reduce(name, args, ops::min),
        "abs" => unary(name, args, f64::abs),
        "sqrt" => unary(name, args, f64::sqrt),
        "transpose" => {
            let a = arg_num(name, args, 0)?;
            Ok(ScilabValue::Num(ops::transpose(a)))
        }
        "disp" => {
            let _ = one_arg(name, args)?;
            Ok(ScilabValue::Num(
                Array::from_shape(vec![], vec![0, 0]).unwrap(),
            )) // invisible
        }
        other => Err(format!(
            "scilab-runtime: '{other}' is not a known function"
        )),
    }
}

/// `zeros(n)` → `n×n`; `zeros(r, c)` → `r×c`.
fn constructor(name: &str, args: &[ScilabValue], fill: f64) -> Result<ScilabValue, String> {
    let (r, c) = dims(name, args)?;
    Ok(ScilabValue::Num(Array::filled(r, c, fill)))
}

/// `eye(n)` → the `n×n` identity.
fn eye(args: &[ScilabValue]) -> Result<ScilabValue, String> {
    let n = count(arg_num("eye", args, 0)?, "eye")?;
    Ok(ScilabValue::Num(Array::eye(n)))
}

/// `size(A)` → the `1×2` row vector `[rows cols]`.
fn size(args: &[ScilabValue]) -> Result<ScilabValue, String> {
    let a = arg_num("size", args, 0)?;
    Array::from_shape(vec![a.nrows() as f64, a.ncols() as f64], vec![1, 2])
        .map(ScilabValue::Num)
}

/// `length(A)` → the largest dimension (0 for an empty array).
fn length(args: &[ScilabValue]) -> Result<ScilabValue, String> {
    let a = arg_num("length", args, 0)?;
    let len = if a.is_empty() {
        0
    } else {
        a.nrows().max(a.ncols())
    };
    Ok(ScilabValue::scalar(len as f64))
}

/// `numel(A)` → the element count.
fn numel(args: &[ScilabValue]) -> Result<ScilabValue, String> {
    Ok(ScilabValue::scalar(arg_num("numel", args, 0)?.len() as f64))
}

/// A whole-array reduction to a scalar.
fn reduce(name: &str, args: &[ScilabValue], f: fn(&Array) -> f64) -> Result<ScilabValue, String> {
    Ok(ScilabValue::scalar(f(arg_num(name, args, 0)?)))
}

/// An element-wise unary math function.
fn unary(name: &str, args: &[ScilabValue], f: fn(f64) -> f64) -> Result<ScilabValue, String> {
    let a = arg_num(name, args, 0)?;
    Array::from_shape(a.data().iter().map(|&x| f(x)).collect(), a.shape().to_vec())
        .map(ScilabValue::Num)
}

// --- argument helpers ----------------------------------------------------

/// Interpret a constructor's arguments: `f(n)` → `(n, n)`, `f(r, c)` → `(r, c)`.
fn dims(name: &str, args: &[ScilabValue]) -> Result<(usize, usize), String> {
    match args {
        [n] => {
            let n = count(n.as_num(name)?, name)?;
            Ok((n, n))
        }
        [r, c] => Ok((count(r.as_num(name)?, name)?, count(c.as_num(name)?, name)?)),
        _ => Err(format!("{name}: expected 1 or 2 size arguments")),
    }
}

/// Read a non-negative dimension count from a scalar array, capped so a
/// crafted `zeros(1e18)` is a clean error rather than an allocation abort.
fn count(a: &Array, name: &str) -> Result<usize, String> {
    const MAX_DIM: f64 = (1u64 << 26) as f64;
    let x = a.data().first().copied().unwrap_or(0.0);
    if !(0.0..=MAX_DIM).contains(&x) {
        return Err(format!("{name}: size must be in 0..={}", MAX_DIM as u64));
    }
    Ok(x.round() as usize)
}

fn one_arg<'a>(name: &str, args: &'a [ScilabValue]) -> Result<&'a ScilabValue, String> {
    args.first()
        .ok_or_else(|| format!("{name}: expected an argument"))
}

fn arg_num<'a>(name: &str, args: &'a [ScilabValue], i: usize) -> Result<&'a Array, String> {
    args.get(i)
        .ok_or_else(|| format!("{name}: expected at least {} argument(s)", i + 1))?
        .as_num(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_eight_percent_constants_resolve() {
        assert_eq!(
            percent_const("%pi").unwrap().as_num("t").unwrap().data()[0],
            std::f64::consts::PI
        );
        assert_eq!(
            percent_const("%e").unwrap().as_num("t").unwrap().data()[0],
            std::f64::consts::E
        );
        assert!(percent_const("%i").is_err());
        assert!(percent_const("%inf")
            .unwrap()
            .as_num("t")
            .unwrap()
            .data()[0]
            .is_infinite());
        assert!(percent_const("%nan")
            .unwrap()
            .as_num("t")
            .unwrap()
            .data()[0]
            .is_nan());
        assert_eq!(
            percent_const("%eps").unwrap().as_num("t").unwrap().data()[0],
            f64::EPSILON
        );
        assert_eq!(
            percent_const("%t").unwrap().as_num("t").unwrap().data()[0],
            1.0
        );
        assert_eq!(
            percent_const("%f").unwrap().as_num("t").unwrap().data()[0],
            0.0
        );
    }

    #[test]
    fn zeros_ones_eye_construct_arrays() {
        let z = call("zeros", &[ScilabValue::scalar(2.0)]).unwrap();
        assert_eq!(z.as_num("t").unwrap().shape(), &[2, 2]);
        let o = call("ones", &[ScilabValue::scalar(2.0), ScilabValue::scalar(3.0)]).unwrap();
        assert_eq!(o.as_num("t").unwrap().shape(), &[2, 3]);
        let e = call("eye", &[ScilabValue::scalar(3.0)]).unwrap();
        assert_eq!(ops::sum(e.as_num("t").unwrap()), 3.0);
    }

    #[test]
    fn unknown_builtin_is_an_error() {
        assert!(call("not_a_real_function", &[]).is_err());
    }

    #[test]
    fn builtins_reject_a_string_argument() {
        assert!(call("sqrt", &[ScilabValue::Str("x".to_string())]).is_err());
    }
}
