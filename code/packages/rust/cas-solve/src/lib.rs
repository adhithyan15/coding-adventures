//! # cas-solve
//!
//! Closed-form equation solving over ℚ (Phase 1: linear and quadratic).
//!
//! ## Quick start
//!
//! ```rust
//! use cas_solve::{solve_linear, solve_quadratic, SolveResult};
//! use cas_solve::frac::Frac;
//! use symbolic_ir::{int, rat};
//!
//! // 2x + 3 = 0  →  x = -3/2
//! let r = solve_linear(Frac::from_int(2), Frac::from_int(3));
//! assert_eq!(r, SolveResult::Solutions(vec![rat(-3, 2)]));
//!
//! // x^2 - 5x + 6 = 0  →  {2, 3}
//! let r2 = solve_quadratic(
//!     Frac::from_int(1), Frac::from_int(-5), Frac::from_int(6),
//! );
//! assert_eq!(r2, SolveResult::Solutions(vec![int(2), int(3)]));
//! ```
//!
//! ## IR head names
//!
//! | Constant | Value |
//! |----------|-------|
//! | [`SOLVE`] | `"Solve"` |
//! | [`NSOLVE`] | `"NSolve"` |
//! | [`ROOTS`] | `"Roots"` |

pub mod frac;
pub mod linear;
pub mod quadratic;

pub use linear::solve_linear;
pub use quadratic::{solve_quadratic, I_UNIT};

/// The result of an equation-solve operation.
///
/// - `Solutions(vec)` — zero, one, or two solutions (empty = no solution).
/// - `All` — every value of x satisfies the equation.
#[derive(Debug, Clone, PartialEq)]
pub enum SolveResult {
    Solutions(Vec<symbolic_ir::IRNode>),
    All,
}

/// Head symbol for the `Solve(expr, var)` form.
pub const SOLVE: &str = "Solve";

/// Head symbol for numeric solving (future Phase 2).
pub const NSOLVE: &str = "NSolve";

/// Head symbol for root-finding operations (future Phase 2).
pub const ROOTS: &str = "Roots";
