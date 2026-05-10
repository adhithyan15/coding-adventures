//! # cas-multivariate
//!
//! Sparse multivariate polynomials over Q, polynomial reduction, Groebner
//! bases, and small ideal solving via lex order back-substitution.

pub mod groebner;
pub mod monomial;
pub mod polynomial;
pub mod rational;
pub mod reduce;
pub mod solve;

pub use groebner::{buchberger, GrobnerError};
pub use monomial::{
    cmp_monomials, div_monomial, divides, lcm_monomial, total_degree, Monomial, MonomialOrder,
    MonomialOrderError,
};
pub use polynomial::{div_reduction_step, make_var, MPoly, PolynomialError};
pub use rational::Rational;
pub use reduce::{reduce_poly, s_poly};
pub use solve::{ideal_solve, ideal_solve_with_order, rational_roots, solve_univariate};

/// Head symbol for Groebner basis operations.
pub const GROEBNER: &str = "Groebner";

/// Head symbol for polynomial normal-form reduction.
pub const POLY_REDUCE: &str = "PolyReduce";

/// Head symbol for ideal solving.
pub const IDEAL_SOLVE: &str = "IdealSolve";
