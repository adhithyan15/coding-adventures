//! # cas-factor
//!
//! Univariate integer polynomial factoring over ℤ (Phase 1: linear factors
//! via the rational-root test).
//!
//! ## Quick start
//!
//! ```rust
//! use cas_factor::factor_integer_polynomial;
//!
//! // x^2 - 1 = (x - 1)(x + 1)
//! let (content, factors) = factor_integer_polynomial(&[-1, 0, 1]);
//! assert_eq!(content, 1);
//! assert_eq!(factors.len(), 2);  // two linear factors
//!
//! // 2*(x+1)^2
//! let (c, f) = factor_integer_polynomial(&[2, 4, 2]);
//! assert_eq!(c, 2);
//! assert_eq!(f, vec![(vec![1, 1], 2)]);
//! ```
//!
//! ## Polynomial representation
//!
//! Polynomials are `Vec<i64>` coefficient lists with the constant term first:
//! `[a_0, a_1, ..., a_n]` represents `a_0 + a_1·x + … + a_n·x^n`.
//!
//! ## What Phase 1 covers
//!
//! Phase 1 uses the Rational Root Theorem to find all **integer** roots.
//! Any irreducible residual (e.g., `x^2 + 1`) is appended with multiplicity 1.
//! Full Kronecker factorization and factoring over ℚ is Phase 2.
//!
//! ## IR head names
//!
//! The `symbolic-ir` integration layer uses these names as Apply heads:
//!
//! | Constant | Value |
//! |----------|-------|
//! | [`FACTOR`] | `"Factor"` |
//! | [`IRREDUCIBLE`] | `"Irreducible"` |

pub mod factor;
pub mod polynomial;
pub mod rational_roots;

pub use factor::{factor_integer_polynomial, FactorList};
pub use polynomial::{
    content, degree, divide_linear, divisors, evaluate, normalize, primitive_part, Poly,
};
pub use rational_roots::{extract_linear_factors, find_integer_roots};

/// Head symbol for the Factor form: `Factor(expr)`.
pub const FACTOR: &str = "Factor";

/// Head symbol for residuals that could not be factored further in Phase 1.
pub const IRREDUCIBLE: &str = "Irreducible";
