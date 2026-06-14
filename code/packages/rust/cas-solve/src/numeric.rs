//! Numeric polynomial root-finding via Durand-Kerner iteration.
//!
//! Coefficients are supplied in decreasing degree order, matching the Python
//! and TypeScript `cas-solve` ports: `[a_n, a_{n-1}, ..., a_0]`.

use std::f64::consts::TAU;

use symbolic_ir::{apply, flt, sym, IRNode, ADD, MUL};

use crate::frac::Frac;
use crate::quadratic::I_UNIT;

/// A lightweight complex number used by the pure Rust numeric solver.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    pub const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    pub fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }
}

/// Find all roots of a polynomial numerically with Durand-Kerner iteration.
///
/// The polynomial is normalised by its non-zero leading coefficient before
/// iteration. Constant polynomials return an empty root list.
///
/// # Panics
///
/// Panics when the leading coefficient is zero.
pub fn nsolve_poly(coeffs: &[Complex], max_iter: usize, tol: f64) -> Vec<Complex> {
    let degree = coeffs.len().saturating_sub(1);
    if degree == 0 {
        return Vec::new();
    }

    let lead = coeffs[0];
    assert!(
        lead.abs() != 0.0,
        "nsolve_poly: leading coefficient must not be zero"
    );
    let poly: Vec<Complex> = coeffs.iter().map(|coef| *coef / lead).collect();

    if degree == 1 {
        return vec![-poly[1]];
    }

    let radius = initial_radius(&poly);
    let mut roots: Vec<Complex> = (0..degree)
        .map(|k| {
            let theta = TAU * (k as f64) / (degree as f64) + 0.1;
            Complex::new(radius * theta.cos(), radius * theta.sin())
        })
        .collect();

    for _ in 0..max_iter {
        let mut max_delta = 0.0_f64;
        let mut next = roots.clone();

        for i in 0..degree {
            let mut denom = Complex::new(1.0, 0.0);
            for j in 0..degree {
                if i == j {
                    continue;
                }
                let mut diff = roots[i] - roots[j];
                if diff.abs() < 1e-300 {
                    diff = Complex::new(1e-300, 0.0);
                }
                denom = denom * diff;
            }

            let delta = eval_poly(&poly, roots[i]) / denom;
            next[i] = roots[i] - delta;
            max_delta = max_delta.max(delta.abs());
        }

        roots = next;
        if max_delta < tol {
            break;
        }
    }

    roots
}

/// Convert numeric roots to symbolic IR nodes.
///
/// Nearly-real roots become `Float(re)`. Complex roots become
/// `Add(Float(re), Mul(Float(im), %i))`, preserving the MACSYMA imaginary-unit
/// convention used by the exact solvers.
pub fn roots_to_ir(roots: &[Complex]) -> Vec<IRNode> {
    roots
        .iter()
        .map(|root| {
            if root.im.abs() < 1e-10 {
                flt(root.re)
            } else {
                apply(
                    sym(ADD),
                    vec![
                        flt(root.re),
                        apply(sym(MUL), vec![flt(root.im), sym(I_UNIT)]),
                    ],
                )
            }
        })
        .collect()
}

/// Convenience wrapper for exact rational coefficient input.
pub fn nsolve_fraction_poly(coeffs: &[Frac]) -> Vec<IRNode> {
    let numeric: Vec<Complex> = coeffs
        .iter()
        .map(|coef| Complex::new(coef.numer as f64 / coef.denom as f64, 0.0))
        .collect();
    roots_to_ir(&nsolve_poly(&numeric, 200, 1e-12))
}

fn eval_poly(poly: &[Complex], z: Complex) -> Complex {
    poly.iter()
        .fold(Complex::new(0.0, 0.0), |acc, coef| acc * z + *coef)
}

fn initial_radius(poly: &[Complex]) -> f64 {
    let degree = poly.len().saturating_sub(1);
    if degree == 0 {
        return 1.0;
    }
    let cauchy = 1.0 + poly[1..].iter().map(|coef| coef.abs()).fold(0.0, f64::max);
    let lagrange = if poly[degree].abs() > 1e-300 {
        poly[degree].abs().powf(1.0 / degree as f64)
    } else {
        1.0
    };
    cauchy.min(10.0).max(lagrange).max(0.5)
}

impl std::ops::Add for Complex {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.re + rhs.re, self.im + rhs.im)
    }
}

impl std::ops::Sub for Complex {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.re - rhs.re, self.im - rhs.im)
    }
}

impl std::ops::Neg for Complex {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.re, -self.im)
    }
}

impl std::ops::Mul for Complex {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(
            self.re * rhs.re - self.im * rhs.im,
            self.re * rhs.im + self.im * rhs.re,
        )
    }
}

impl std::ops::Div for Complex {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        let denom = rhs.re * rhs.re + rhs.im * rhs.im;
        assert!(denom != 0.0, "complex division by zero");
        Self::new(
            (self.re * rhs.re + self.im * rhs.im) / denom,
            (self.im * rhs.re - self.re * rhs.im) / denom,
        )
    }
}
