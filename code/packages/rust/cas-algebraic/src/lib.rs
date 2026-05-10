//! Quadratic algebraic extension factoring for symbolic CAS polynomials.
//!
//! This crate ports the Python `cas-algebraic` package's first factoring
//! slice to Rust. It factors integer univariate polynomials over `Q[sqrt(d)]`
//! for monic quadratics and depressed monic quartics, after first using
//! `cas-factor` to split any integer factors already visible over `Z`.

pub mod algebraic;
pub mod ir;
pub mod rational;

pub use algebraic::{
    factor_over_extension, try_split_depressed_quartic, try_split_quadratic, try_split_single,
    AlgCoeff, AlgPoly,
};
pub use ir::{
    alg_coeff_to_ir, alg_factor_ir, alg_poly_to_ir, extract_radical_d, factors_to_ir,
    ir_to_integer_poly, ALG_FACTOR,
};
pub use rational::{rational_square_root, Rational};
