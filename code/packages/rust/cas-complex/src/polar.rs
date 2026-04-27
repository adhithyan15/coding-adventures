//! Polar form operations: modulus and argument.
//!
//! For a complex number `z = a + b·i`, the polar form is:
//!
//! ```text
//! z = r · e^{iθ}
//! ```
//!
//! where:
//! - `r = |z| = √(a² + b²)` is the **modulus** (non-negative real),
//! - `θ = arg(z) = atan2(b, a)` is the **argument** in `(−π, π]`.
//!
//! # Symbolic vs. numeric
//!
//! For numeric inputs (`IRNode::Integer` / `IRNode::Float` / `IRNode::Rational`
//! real and imaginary parts), the functions return exact `IRNode::Float` values.
//!
//! For symbolic inputs, the functions return unevaluated `Abs(z)` and `Arg(z)`
//! nodes respectively — letting the caller decide how to further simplify.
//!
//! # Special exact cases
//!
//! | `z` | `|z|` | `arg(z)` |
//! |-----|-------|---------|
//! | `0`   | `0.0`     | `0.0` |
//! | real `r > 0` | `r` (exact) | `0.0` |
//! | real `r < 0` | `-r` (exact) | `π` |
//! | `i`   | `1.0` | `π/2` |
//! | `-i`  | `1.0` | `-π/2` |

use symbolic_ir::{apply, flt, sym, IRNode};

use crate::constants::{ABS, ARG};
use crate::normalize::split_complex;

// ---------------------------------------------------------------------------
// Modulus
// ---------------------------------------------------------------------------

/// Return the modulus `|z| = √(a² + b²)` of `z`.
///
/// If both parts are numeric, returns an `IRNode::Float`.
/// Otherwise returns `Abs(z)`.
///
/// # Examples
///
/// ```rust
/// use cas_complex::modulus;
/// use symbolic_ir::{apply, int, sym, ADD, MUL};
///
/// // |3 + 4*I| = 5.0
/// let z = apply(sym(ADD), vec![int(3), apply(sym(MUL), vec![int(4), sym("I")])]);
/// if let symbolic_ir::IRNode::Float(v) = modulus(&z) {
///     assert!((v - 5.0).abs() < 1e-10);
/// }
///
/// // |0| = 0.0
/// if let symbolic_ir::IRNode::Float(v) = modulus(&int(0)) {
///     assert_eq!(v, 0.0);
/// }
/// ```
pub fn modulus(expr: &IRNode) -> IRNode {
    let (re, im) = split_complex(expr);
    match (to_float(&re), to_float(&im)) {
        (Some(a), Some(b)) => flt((a * a + b * b).sqrt()),
        _ => apply(sym(ABS), vec![expr.clone()]),
    }
}

// ---------------------------------------------------------------------------
// Argument
// ---------------------------------------------------------------------------

/// Return the argument (phase angle) `θ = atan2(b, a)` of `z = a + b·i`.
///
/// Result is in `(−π, π]`.  If both parts are numeric, returns
/// `IRNode::Float`.  Otherwise returns `Arg(z)`.
///
/// # Examples
///
/// ```rust
/// use cas_complex::argument;
/// use symbolic_ir::{apply, int, sym, ADD, MUL};
/// use std::f64::consts::{FRAC_PI_2, PI};
///
/// // arg(i) = π/2
/// if let symbolic_ir::IRNode::Float(v) = argument(&sym("I")) {
///     assert!((v - FRAC_PI_2).abs() < 1e-10);
/// }
///
/// // arg(-1) = π
/// if let symbolic_ir::IRNode::Float(v) = argument(&int(-1)) {
///     assert!((v - PI).abs() < 1e-10);
/// }
/// ```
pub fn argument(expr: &IRNode) -> IRNode {
    let (re, im) = split_complex(expr);
    match (to_float(&re), to_float(&im)) {
        (Some(a), Some(b)) => flt(b.atan2(a)),
        _ => apply(sym(ARG), vec![expr.clone()]),
    }
}

// ---------------------------------------------------------------------------
// Helper: try to extract a float from a numeric IRNode
// ---------------------------------------------------------------------------

/// Convert `n` to `f64` if it is a numeric literal, otherwise return `None`.
fn to_float(n: &IRNode) -> Option<f64> {
    match n {
        IRNode::Integer(v) => Some(*v as f64),
        IRNode::Float(v) => Some(*v),
        IRNode::Rational(numer, denom) => Some(*numer as f64 / *denom as f64),
        _ => None,
    }
}
