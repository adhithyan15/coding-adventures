//! The core MATLAB built-in functions, over `array-runtime`.
//!
//! This is the starter set a textbook session needs: array constructors
//! (`zeros`/`ones`/`eye`), shape queries (`size`/`length`/`numel`), the
//! whole-array reductions (`sum`/`mean`/`max`/`min`), element-wise math
//! (`abs`/`sqrt`), `transpose`, and `disp`. More of the library follows; this is
//! the seam each addition slots into.

use crate::value::MatValue;
use array_runtime::{ops, Array};

/// Dispatch a builtin by name. Returns `Err` for an unknown name (which the
/// evaluator reports as an undefined function/variable).
pub fn call(name: &str, args: &[MatValue]) -> Result<MatValue, String> {
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
            Ok(MatValue::Num(ops::transpose(a)))
        }
        "disp" => {
            let _ = one_arg(name, args)?;
            Ok(MatValue::Num(
                Array::from_shape(vec![], vec![0, 0]).unwrap(),
            )) // invisible
        }
        other => Err(format!("matlab-runtime: '{other}' is not a known function")),
    }
}

/// `zeros(n)` → `n×n`; `zeros(r, c)` → `r×c`.
fn constructor(name: &str, args: &[MatValue], fill: f64) -> Result<MatValue, String> {
    let (r, c) = dims(name, args)?;
    Ok(MatValue::Num(Array::filled(r, c, fill)))
}

/// `eye(n)` → the `n×n` identity.
fn eye(args: &[MatValue]) -> Result<MatValue, String> {
    let n = count(arg_num("eye", args, 0)?, "eye")?;
    check_total_elements("eye", n, n)?;
    Ok(MatValue::Num(Array::eye(n)))
}

/// `size(A)` → the `1×2` row vector `[rows cols]`.
fn size(args: &[MatValue]) -> Result<MatValue, String> {
    let a = arg_num("size", args, 0)?;
    Array::from_shape(vec![a.nrows() as f64, a.ncols() as f64], vec![1, 2]).map(MatValue::Num)
}

/// `length(A)` → the largest dimension (0 for an empty array).
fn length(args: &[MatValue]) -> Result<MatValue, String> {
    let a = arg_num("length", args, 0)?;
    let len = if a.is_empty() {
        0
    } else {
        a.nrows().max(a.ncols())
    };
    Ok(MatValue::scalar(len as f64))
}

/// `numel(A)` → the element count.
fn numel(args: &[MatValue]) -> Result<MatValue, String> {
    Ok(MatValue::scalar(arg_num("numel", args, 0)?.len() as f64))
}

/// A whole-array reduction to a scalar.
fn reduce(name: &str, args: &[MatValue], f: fn(&Array) -> f64) -> Result<MatValue, String> {
    Ok(MatValue::scalar(f(arg_num(name, args, 0)?)))
}

/// An element-wise unary math function.
fn unary(name: &str, args: &[MatValue], f: fn(f64) -> f64) -> Result<MatValue, String> {
    let a = arg_num(name, args, 0)?;
    Array::from_shape(a.data().iter().map(|&x| f(x)).collect(), a.shape().to_vec())
        .map(MatValue::Num)
}

// --- argument helpers ----------------------------------------------------

/// Interpret a constructor's arguments: `f(n)` → `(n, n)`, `f(r, c)` → `(r, c)`.
fn dims(name: &str, args: &[MatValue]) -> Result<(usize, usize), String> {
    let (r, c) = match args {
        [n] => {
            let n = count(n.as_num(name)?, name)?;
            (n, n)
        }
        [r, c] => (count(r.as_num(name)?, name)?, count(c.as_num(name)?, name)?),
        _ => return Err(format!("{name}: expected 1 or 2 size arguments")),
    };
    // Each dimension alone is within `count()`'s own per-dimension cap, but
    // their PRODUCT is not -- `zeros(1<<26, 1<<26)` would otherwise request
    // an allocation of ~4.5e15 elements before this ever runs. Security
    // regression, mirrors the fix already shipped in scilab-runtime.
    check_total_elements(name, r, c)?;
    Ok((r, c))
}

/// The total-element cap every array-constructing/-growing operation must
/// enforce on its FINAL size -- not just on each dimension independently.
/// Capping each dimension alone (as `count()` already does) does not bound
/// their PRODUCT: two independently-in-bounds dimensions, or two
/// independently-in-bounds index vectors used as `A(idx, idx)`, can still
/// combine into an astronomical total. Uses `checked_mul` so the
/// multiplication itself cannot silently overflow before the comparison
/// runs.
pub(crate) const MAX_TOTAL_ELEMENTS: usize = 1 << 26;

pub(crate) fn check_total_elements(name: &str, rows: usize, cols: usize) -> Result<(), String> {
    match rows.checked_mul(cols) {
        Some(total) if total <= MAX_TOTAL_ELEMENTS => Ok(()),
        _ => Err(format!(
            "{name}: {rows}x{cols} exceeds the {MAX_TOTAL_ELEMENTS}-element limit"
        )),
    }
}

/// Read a non-negative dimension count from a scalar array, capped so a crafted
/// `zeros(1e18)` is a clean error rather than an allocation abort.
fn count(a: &Array, name: &str) -> Result<usize, String> {
    const MAX_DIM: f64 = (1u64 << 26) as f64;
    let x = a.data().first().copied().unwrap_or(0.0);
    if !(0.0..=MAX_DIM).contains(&x) {
        return Err(format!("{name}: size must be in 0..={}", MAX_DIM as u64));
    }
    Ok(x.round() as usize)
}

fn one_arg<'a>(name: &str, args: &'a [MatValue]) -> Result<&'a MatValue, String> {
    args.first()
        .ok_or_else(|| format!("{name}: expected an argument"))
}

fn arg_num<'a>(name: &str, args: &'a [MatValue], i: usize) -> Result<&'a Array, String> {
    args.get(i)
        .ok_or_else(|| format!("{name}: expected at least {} argument(s)", i + 1))?
        .as_num(name)
}
