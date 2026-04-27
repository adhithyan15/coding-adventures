//! # cas-limit-series
//!
//! Limit (direct substitution) and polynomial Taylor series over symbolic IR.
//!
//! Rust port of the Python `cas-limit-series` package.
//!
//! ## Limit
//!
//! `limit_direct` computes `lim_{var→point} expr` by substituting `point` for
//! `var`.  It does **not** simplify the result — pass it through
//! `cas_simplify::simplify` afterwards.  The only indeterminate form detected
//! is a literal `Div(0, 0)`, which is returned as an unevaluated
//! `Limit(expr, var, point)` node.
//!
//! ```rust
//! use cas_limit_series::limit_direct;
//! use symbolic_ir::{apply, int, sym, ADD, MUL};
//!
//! // lim_{x→3} 2*x  →  Mul(2, 3)  (un-simplified)
//! let x = sym("x");
//! let expr = apply(sym(MUL), vec![int(2), x.clone()]);
//! let out = limit_direct(expr, &x, int(3));
//! assert_eq!(out, apply(sym(MUL), vec![int(2), int(3)]));
//! ```
//!
//! ## Taylor series (polynomial inputs)
//!
//! `taylor_polynomial` builds the truncated Taylor expansion of a polynomial
//! expression around a numeric point:
//!
//! ```text
//! sum_{k=0..order}  (1/k!) · p^(k)(point) · (var − point)^k
//! ```
//!
//! Only polynomial inputs are accepted (`Add`, `Mul`, `Pow` with non-negative
//! integer exponents, `Sub`, `Neg`, numeric literals).  Transcendental
//! functions raise [`PolynomialError`].
//!
//! ```rust
//! use cas_limit_series::taylor_polynomial;
//! use symbolic_ir::{apply, int, sym, POW};
//!
//! // Taylor(x^2, x, 0, order=2)  →  Pow(x, 2)
//! let x = sym("x");
//! let expr = apply(sym(POW), vec![x.clone(), int(2)]);
//! let out = taylor_polynomial(&expr, &x, &int(0), 2).unwrap();
//! assert_eq!(out, apply(sym(POW), vec![x.clone(), int(2)]));
//! ```
//!
//! ## Stack position
//!
//! ```text
//! symbolic-ir  ←  cas-substitution  ←  cas-limit-series
//! ```

pub mod limit;
pub mod taylor;

pub use limit::limit_direct;
pub use taylor::{taylor_polynomial, PolynomialError};

// ---------------------------------------------------------------------------
// IR head-name string constants
// ---------------------------------------------------------------------------

/// Head name for the unevaluated `Limit(expr, var, point)` form.
pub const LIMIT: &str = "Limit";

/// Head name for the `Taylor(expr, var, point, order)` form.
pub const TAYLOR: &str = "Taylor";

/// Head name for the `Series(expr, var, point, order)` form.
pub const SERIES: &str = "Series";

/// Head name for the big-O notation term.
pub const BIG_O: &str = "BigO";
