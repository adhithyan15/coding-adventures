//! Linear-equation closed form: `a·x + b = 0  →  x = −b/a`.

use crate::frac::Frac;
use crate::SolveResult;

/// Solve `a·x + b = 0` over ℚ.
///
/// # Returns
///
/// - [`SolveResult::Solutions`]`(vec![x])` when `a ≠ 0`.
/// - [`SolveResult::Solutions`]`(vec![])` (no solutions) when `a = 0` and `b ≠ 0`.
/// - [`SolveResult::All`] (every x satisfies) when `a = 0` and `b = 0`.
///
/// # Examples
///
/// ```rust
/// use cas_solve::{solve_linear, SolveResult};
/// use cas_solve::frac::Frac;
/// use symbolic_ir::{int, rat};
///
/// // 2x + 3 = 0  →  x = -3/2
/// let result = solve_linear(Frac::from_int(2), Frac::from_int(3));
/// assert_eq!(result, SolveResult::Solutions(vec![rat(-3, 2)]));
///
/// // 0*x + 5 = 0  →  no solution
/// let no_sol = solve_linear(Frac::zero(), Frac::from_int(5));
/// assert_eq!(no_sol, SolveResult::Solutions(vec![]));
///
/// // 0*x + 0 = 0  →  all x satisfy
/// let all = solve_linear(Frac::zero(), Frac::zero());
/// assert_eq!(all, SolveResult::All);
/// ```
pub fn solve_linear(a: Frac, b: Frac) -> SolveResult {
    if a.is_zero() {
        if b.is_zero() {
            return SolveResult::All;
        }
        return SolveResult::Solutions(vec![]);
    }
    // x = -b / a
    let x = (-b) / a;
    SolveResult::Solutions(vec![x.to_irnode()])
}
