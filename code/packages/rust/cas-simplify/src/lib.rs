//! # cas-simplify
//!
//! Algebraic simplification of symbolic IR trees.
//!
//! ## Pipeline
//!
//! ```text
//! canonical  →  numeric_fold  →  identity_rules  →  (repeat to fixed point)
//! ```
//!
//! - **`canonical`** — flatten nested Add/Mul, sort commutative args,
//!   drop singleton Add(x)/Mul(x) → x, collapse empty Add() → 0 / Mul() → 1.
//! - **`numeric_fold`** — accumulate adjacent numeric literals inside Add/Mul
//!   arg lists into a single literal.
//! - **`identity_rules`** — pattern-matching rewrites for standard algebraic
//!   identities (x+0→x, x*1→x, x^0→1, …).
//!
//! ## Example
//!
//! ```rust
//! use symbolic_ir::{apply, int, sym, ADD, MUL, POW};
//! use cas_simplify::simplify;
//!
//! // Add(x, 0) → x
//! let expr = apply(sym(ADD), vec![sym("x"), int(0)]);
//! assert_eq!(simplify(expr, 50), sym("x"));
//!
//! // Mul(2, 3) → 6
//! let expr2 = apply(sym(MUL), vec![int(2), int(3)]);
//! assert_eq!(simplify(expr2, 50), int(6));
//!
//! // Pow(x, 0) → 1
//! let expr3 = apply(sym(POW), vec![sym("x"), int(0)]);
//! assert_eq!(simplify(expr3, 50), int(1));
//! ```

pub mod canonical;
pub mod numeric_fold;
pub mod rules;
pub mod simplifier;

pub use canonical::canonical;
pub use numeric_fold::numeric_fold;
pub use rules::build_identity_rules;
pub use simplifier::simplify;
