//! # cas-solve
//!
//! Closed-form and numeric equation solving over ℚ: linear, quadratic, cubic,
//! quartic, and Durand-Kerner polynomial roots.
//!
//! ## Quick start
//!
//! ```rust
//! use cas_solve::{nsolve_poly, solve_cubic, solve_linear, solve_quadratic, solve_quartic, Complex, SolveResult};
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
//!
//! // x^3 - 6x^2 + 11x - 6 = 0  →  {1, 2, 3}
//! let r3 = solve_cubic(
//!     Frac::from_int(1), Frac::from_int(-6), Frac::from_int(11), Frac::from_int(-6),
//! );
//! assert_eq!(r3, SolveResult::Solutions(vec![int(1), int(2), int(3)]));
//!
//! // x^4 - 5x^2 + 4 = 0  →  {-2, -1, 1, 2}
//! let r4 = solve_quartic(
//!     Frac::from_int(1), Frac::from_int(0), Frac::from_int(-5),
//!     Frac::from_int(0), Frac::from_int(4),
//! );
//! assert!(matches!(r4, SolveResult::Solutions(roots) if roots.contains(&int(-2)) && roots.contains(&int(2))));
//!
//! // Numeric roots for x^2 + 1 = 0.
//! let numeric = nsolve_poly(
//!     &[Complex::new(1.0, 0.0), Complex::new(0.0, 0.0), Complex::new(1.0, 0.0)],
//!     200,
//!     1e-12,
//! );
//! assert_eq!(numeric.len(), 2);
//! ```
//!
//! ## IR head names
//!
//! | Constant | Value |
//! |----------|-------|
//! | [`SOLVE`] | `"Solve"` |
//! | [`NSOLVE`] | `"NSolve"` |
//! | [`ROOTS`] | `"Roots"` |

pub mod cubic;
pub mod frac;
pub mod linear;
pub mod numeric;
pub mod quadratic;
pub mod quartic;

pub use cubic::{solve_cubic, CBRT};
pub use linear::solve_linear;
pub use numeric::{nsolve_fraction_poly, nsolve_poly, roots_to_ir, Complex};
pub use quadratic::{solve_quadratic, I_UNIT};
pub use quartic::solve_quartic;

/// The result of an equation-solve operation.
///
/// - `Solutions(vec)` — zero or more solutions (empty = no solution or
///   unevaluated symbolic fallback, depending on the solver).
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
