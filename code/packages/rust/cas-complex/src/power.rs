//! Complex integer powers via De Moivre's theorem.
//!
//! For a complex number `z = a + b·i` and integer exponent `n`:
//!
//! ```text
//! z^n = (r·e^{iθ})^n = r^n · e^{i·n·θ}
//!      = r^n · (cos(n·θ) + i·sin(n·θ))
//! ```
//!
//! where `r = |z|` and `θ = arg(z)`.
//!
//! For numeric `z` and integer `n`, the power is computed numerically and
//! returned as `a + b·I` (with `IRNode::Float` coefficients).
//!
//! For symbolic `z` or non-integer `n`, the unevaluated `Pow(z, n)` node
//! is returned.
//!
//! # Special integer cases (exact)
//!
//! - `z^0 = 1`
//! - `z^1 = z`
//! - `z^(-1) = conj(z) / |z|²`  (for numeric z with |z| ≠ 0)
//! - `I^n` cycles through `1, i, -1, -i, …` → handled exactly by
//!   [`split_complex`] already.

use symbolic_ir::{apply, flt, int, sym, IRNode, POW};

use crate::normalize::{assemble, split_complex};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Raise `base` to the integer power `n`.
///
/// Returns a normalised complex `a + b·I` result when both `base` and `n`
/// are numeric.  Otherwise returns `Pow(base, n)` unevaluated.
///
/// # Examples
///
/// ```rust
/// use cas_complex::complex_pow;
/// use symbolic_ir::{apply, int, sym, MUL};
///
/// // (1 + I)^2 = 2*I  (i.e. real=0, imag=2)
/// let z = apply(sym(MUL), vec![int(1), int(1)]);  // simplified form; let's use literal
/// let z = apply(sym("Add"), vec![int(1), sym("I")]);
/// let result = complex_pow(&z, &int(2));
/// use cas_complex::imag_part;
/// assert_eq!(imag_part(&result), int(2));
///
/// // I^4 = 1
/// let result2 = complex_pow(&sym("I"), &int(4));
/// assert_eq!(result2, int(1));
/// ```
pub fn complex_pow(base: &IRNode, exp: &IRNode) -> IRNode {
    // Only handle integer exponents.
    let n = match exp {
        IRNode::Integer(n) => *n,
        _ => return apply(sym(POW), vec![base.clone(), exp.clone()]),
    };

    // z^0 = 1 for any z.
    if n == 0 {
        return int(1);
    }

    // z^1 = z.
    if n == 1 {
        return base.clone();
    }

    // Split base into (a, b).
    let (re, im) = split_complex(base);

    // Need numeric parts to evaluate.
    let a = match to_float(&re) {
        Some(v) => v,
        None => return apply(sym(POW), vec![base.clone(), exp.clone()]),
    };
    let b = match to_float(&im) {
        Some(v) => v,
        None => return apply(sym(POW), vec![base.clone(), exp.clone()]),
    };

    // Special case: inverse via conjugate/|z|^2.
    if n == -1 {
        let mag_sq = a * a + b * b;
        if mag_sq == 0.0 {
            // 1/0 — undefined; return unevaluated.
            return apply(sym(POW), vec![base.clone(), exp.clone()]);
        }
        let new_re = a / mag_sq;
        let new_im = -b / mag_sq;
        return assemble_float(new_re, new_im);
    }

    // General case: compute r^n * (cos(nθ) + i*sin(nθ)).
    let r = (a * a + b * b).sqrt();
    let theta = b.atan2(a);

    if n < 0 {
        // z^(-n) = (1/z)^n — use positive exponent on the inverse.
        let mag_sq = a * a + b * b;
        if mag_sq == 0.0 {
            return apply(sym(POW), vec![base.clone(), exp.clone()]);
        }
        let inv_a = a / mag_sq;
        let inv_b = -b / mag_sq;
        let r2 = (inv_a * inv_a + inv_b * inv_b).sqrt();
        let theta2 = inv_b.atan2(inv_a);
        let pos_n = (-n) as u64;
        let rn = r2.powi(pos_n as i32);
        let new_re = rn * (theta2 * pos_n as f64).cos();
        let new_im = rn * (theta2 * pos_n as f64).sin();
        return assemble_float(new_re, new_im);
    }

    let rn = r.powi(n as i32);
    let angle = theta * n as f64;
    let new_re = rn * angle.cos();
    let new_im = rn * angle.sin();
    assemble_float(new_re, new_im)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn to_float(n: &IRNode) -> Option<f64> {
    match n {
        IRNode::Integer(v) => Some(*v as f64),
        IRNode::Float(v) => Some(*v),
        IRNode::Rational(num, den) => Some(*num as f64 / *den as f64),
        _ => None,
    }
}

/// Build a floating-point complex result, snapping near-zero values to
/// exact integer 0 for cleaner output.
fn assemble_float(re: f64, im: f64) -> IRNode {
    let re_node = snap(re);
    let im_node = snap(im);
    assemble(re_node, im_node)
}

/// Snap values very close to an integer (within 1e-9) to that integer.
fn snap(v: f64) -> IRNode {
    let rounded = v.round();
    if (v - rounded).abs() < 1e-9 {
        int(rounded as i64)
    } else {
        flt(v)
    }
}
