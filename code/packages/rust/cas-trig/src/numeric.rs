//! Numeric evaluation of trigonometric functions.
//!
//! When the argument of a trig function is a numeric literal
//! (`Integer`, `Float`, `Rational`) or the symbolic constant `Pi`, the
//! function can be computed numerically using the hardware's double-precision
//! floating-point unit.
//!
//! ## Snapping
//!
//! Results very close to an exact integer (within 1 × 10⁻⁹) are snapped
//! to that integer for clean symbolic output.  For example, `sin(π)` is
//! computed as `≈ 1.2 × 10⁻¹⁶` and snapped to `Integer(0)`.
//!
//! ## Undefinedness
//!
//! `tan` at `±π/2 + k·π` has a pole.  When `|cos(v)| < 1 × 10⁻¹⁵`, the
//! function returns `Float(f64::INFINITY)`.  The caller (`tan_eval` in
//! `simplify.rs`) treats this as "unevaluated" and returns the original
//! `Tan(arg)` expression.

use symbolic_ir::{flt, int, IRNode};

use crate::constants::PI_SYMBOL;

// ---------------------------------------------------------------------------
// Float extraction
// ---------------------------------------------------------------------------

/// Try to convert `n` to `f64`.
///
/// Recognises `Integer`, `Float`, `Rational`, and `Symbol("Pi")`.
/// Returns `None` for symbolic expressions.
pub fn to_float(n: &IRNode) -> Option<f64> {
    match n {
        IRNode::Integer(v) => Some(*v as f64),
        IRNode::Float(v) => Some(*v),
        IRNode::Rational(num, den) => Some(*num as f64 / *den as f64),
        IRNode::Symbol(s) if s.as_str() == PI_SYMBOL => Some(std::f64::consts::PI),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Numeric trig functions
// ---------------------------------------------------------------------------

/// Compute `sin(v)` numerically, snapping near-integers to exact values.
pub fn sin_numeric(v: f64) -> IRNode {
    snap(v.sin())
}

/// Compute `cos(v)` numerically, snapping near-integers to exact values.
pub fn cos_numeric(v: f64) -> IRNode {
    snap(v.cos())
}

/// Compute `tan(v)` numerically.
///
/// Returns `Float(f64::INFINITY)` at poles (`|cos v| < 1e-15`); the
/// caller is responsible for turning this back into an unevaluated node.
pub fn tan_numeric(v: f64) -> IRNode {
    let c = v.cos();
    if c.abs() < 1e-15 {
        flt(f64::INFINITY)
    } else {
        snap(v.sin() / c)
    }
}

/// Compute `atan(v)` numerically.
pub fn atan_numeric(v: f64) -> IRNode {
    flt(v.atan())
}

/// Compute `asin(v)` numerically.
///
/// Returns `Float(f64::NAN)` for `|v| > 1` (out of domain); the caller
/// returns the unevaluated `Asin(arg)` in that case.
pub fn asin_numeric(v: f64) -> IRNode {
    if v.abs() > 1.0 {
        flt(f64::NAN)
    } else {
        flt(v.asin())
    }
}

/// Compute `acos(v)` numerically.
///
/// Returns `Float(f64::NAN)` for `|v| > 1` (out of domain).
pub fn acos_numeric(v: f64) -> IRNode {
    if v.abs() > 1.0 {
        flt(f64::NAN)
    } else {
        flt(v.acos())
    }
}

// ---------------------------------------------------------------------------
// Snapping helper
// ---------------------------------------------------------------------------

/// Snap a floating-point value to a nearby integer when the deviation is
/// within 1 × 10⁻⁹ of that integer.
///
/// This eliminates noise like `sin(π) ≈ 1.2e-16` → `Integer(0)` and
/// `cos(π) ≈ -0.9999999999 → Integer(-1)`.
///
/// The threshold 1e-9 is intentionally much larger than machine epsilon
/// (≈ 2.2e-16) but small enough that legitimate fractional values (e.g.
/// `sin(1.0) ≈ 0.841`) are never rounded.
pub fn snap(v: f64) -> IRNode {
    let rounded = v.round();
    // Avoid NaN comparisons: NaN < 1e-9 is false, so NaN falls through to flt(v).
    if (v - rounded).abs() < 1e-9 {
        int(rounded as i64)
    } else {
        flt(v)
    }
}
