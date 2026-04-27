//! # cas-trig
//!
//! Symbolic trigonometry operations over the shared CAS IR.
//!
//! ## Capabilities
//!
//! | Feature | Description |
//! |---------|-------------|
//! | **Special values** | `sin(π/6)`, `cos(π/4)`, `tan(π/3)` etc. returned as exact `Integer`, `Rational`, or `Sqrt(…)` IR nodes |
//! | **Numeric evaluation** | Any fully-numeric argument (Integer, Float, Rational, or `Pi`) → `Float` with near-integer snapping |
//! | **Angle addition** | `sin(a±b)` and `cos(a±b)` expanded via the angle-addition identities (opt-in via `expand_trig`) |
//! | **Power reduction** | `sin²(x)` and `cos²(x)` rewritten to half-angle forms (opt-in via `power_reduce`) |
//! | **Tree walker** | `trig_simplify` applies evaluation rules everywhere in an expression tree |
//!
//! ## Special-value table
//!
//! Exact values are returned for all rational multiples `n/d · π` where
//! `d ∈ {1, 2, 3, 4, 6}` after reduction modulo 2π:
//!
//! ```text
//! sin(0)   = 0      cos(0)   = 1      tan(0)   = 0
//! sin(π/6) = 1/2    cos(π/6) = √3/2   tan(π/6) = √3/3
//! sin(π/4) = √2/2   cos(π/4) = √2/2   tan(π/4) = 1
//! sin(π/3) = √3/2   cos(π/3) = 1/2    tan(π/3) = √3
//! sin(π/2) = 1      cos(π/2) = 0      tan(π/2) = ∞ (unevaluated)
//! sin(π)   = 0      cos(π)   = −1     tan(π)   = 0
//! …        (and all reflections in [π, 2π) by symmetry)
//! ```
//!
//! ## Quick start
//!
//! ```rust
//! use cas_trig::{sin_eval, cos_eval, tan_eval, trig_simplify, expand_trig,
//!                power_reduce, PI};
//! use symbolic_ir::{apply, int, rat, sym, IRNode, ADD, COS, MUL, POW, SIN};
//!
//! // sin(π/6) = 1/2 (exact)
//! let pi_6 = apply(sym(MUL), vec![rat(1, 6), sym(PI)]);
//! assert_eq!(sin_eval(&pi_6), rat(1, 2));
//!
//! // cos(π) = -1 (exact)
//! assert_eq!(cos_eval(&sym(PI)), int(-1));
//!
//! // trig_simplify descends into any expression
//! let expr = apply(sym(ADD), vec![
//!     apply(sym(SIN), vec![int(0)]),
//!     apply(sym(COS), vec![sym(PI)]),
//! ]);
//! let r = trig_simplify(&expr);
//! if let IRNode::Apply(a) = &r {
//!     assert_eq!(a.args[0], int(0));
//!     assert_eq!(a.args[1], int(-1));
//! }
//!
//! // Expand sin(x + y)
//! let sin_sum = apply(sym(SIN), vec![apply(sym(ADD), vec![sym("x"), sym("y")])]);
//! let _expanded = expand_trig(&sin_sum);
//!
//! // Reduce sin²(x)
//! let sin_sq = apply(sym(POW), vec![apply(sym(SIN), vec![sym("x")]), int(2)]);
//! let _reduced = power_reduce(&sin_sq);
//! ```

pub mod constants;
pub mod expand;
pub mod numeric;
pub mod reduce;
pub mod simplify;
pub mod special;

// Flat re-exports for the most-used items.
pub use constants::{E_SYMBOL as E, PI_SYMBOL as PI};
pub use expand::expand_trig;
pub use numeric::to_float;
pub use reduce::power_reduce;
pub use simplify::{
    acos_eval, asin_eval, atan_eval, cos_eval, extract_pi_multiple, sin_eval, tan_eval,
    trig_simplify,
};
