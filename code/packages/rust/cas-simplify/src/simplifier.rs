//! Fixed-point simplification pipeline.
//!
//! The three passes run in sequence on every iteration:
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────┐
//! │ while changed and iterations < max_iterations             │
//! │   expr = canonical(expr)     ← flatten, sort, singletons │
//! │   expr = numeric_fold(expr)  ← fold literal clusters      │
//! │   expr = rewrite(expr, IDENTITY_RULES, 200)               │
//! └───────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Why a loop?
//!
//! Each pass may expose work for the next.  For example:
//!
//! ```text
//! Mul(Add(x, 0), 1)
//!   → canonical →  Mul(Add(0, x), 1)    (sorts commutative args)
//!   → rewrite   →  Mul(x, 1)            (Add(0, x) → x)
//!   → canonical →  Mul(x, 1)            (already canonical)
//!   → rewrite   →  x                    (Mul(x, 1) → x)
//! ```
//!
//! Two iterations suffice here; the loop terminates as soon as no pass
//! changes the expression.  `max_iterations` is a safety valve.

use cas_pattern_matching::rewrite;
use symbolic_ir::IRNode;

use crate::canonical::canonical;
use crate::numeric_fold::numeric_fold;
use crate::rules::build_identity_rules;

/// Apply `canonical → numeric_fold → identity_rules` to a fixed point.
///
/// # Parameters
///
/// - `expr` — the expression to simplify.
/// - `max_iterations` — maximum number of outer loop iterations.
///   In practice 2–4 suffice; 50 is a safe default.
///
/// # Returns
///
/// The simplified expression.  If the loop reaches `max_iterations` without
/// converging, the best result found so far is returned rather than
/// panicking.
///
/// # Panics
///
/// Does not panic.  A [`RewriteCycleError`] from the inner rewrite step
/// (indicating a divergent rule database) breaks the loop early and returns
/// the last stable expression.
///
/// # Examples
///
/// ```rust
/// use symbolic_ir::{apply, int, sym, ADD, MUL, POW};
/// use cas_simplify::simplify;
///
/// // Add(x, 0) → x
/// let expr = apply(sym(ADD), vec![sym("x"), int(0)]);
/// assert_eq!(simplify(expr, 50), sym("x"));
///
/// // Mul(2, 3) → 6
/// let expr2 = apply(sym(MUL), vec![int(2), int(3)]);
/// assert_eq!(simplify(expr2, 50), int(6));
///
/// // Pow(x, 0) → 1
/// let expr3 = apply(sym(POW), vec![sym("x"), int(0)]);
/// assert_eq!(simplify(expr3, 50), int(1));
/// ```
pub fn simplify(expr: IRNode, max_iterations: usize) -> IRNode {
    // Build the identity rule list once per `simplify` call.  This is a small,
    // fixed-size allocation that is cheap compared to the tree traversals.
    let rules = build_identity_rules();
    let mut current = expr;

    for _ in 0..max_iterations {
        let prev = current.clone();

        // Pass 1: structural canonicalization — flatten nested Add/Mul,
        // sort commutative args, drop singletons, replace empty containers.
        current = canonical(current);

        // Pass 2: constant folding — collapse adjacent numeric literals.
        current = numeric_fold(current);

        // Pass 3: algebraic identity rewrites — bottom-up fixed-point.
        //
        // We clone `current` before passing it to `rewrite` so that the
        // `Err` branch can still return the pre-rewrite state (`prev`) rather
        // than nothing.
        match rewrite(current, &rules, 200) {
            Ok(simplified) => current = simplified,
            Err(_) => {
                // The rule database is diverging — stop early and return the
                // last successfully simplified expression.
                return prev;
            }
        }

        // Converged: this pass changed nothing.
        if current == prev {
            return current;
        }
    }

    current
}
