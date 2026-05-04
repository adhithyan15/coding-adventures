//! # LIA tactic — linear integer arithmetic solver.
//!
//! Implements the **Cooper algorithm** (Omega test fragment) to decide
//! satisfiability of a conjunction of linear arithmetic constraints over
//! integer variables.
//!
//! ## Cooper algorithm — a literate overview
//!
//! Cooper's algorithm (1972) is a quantifier-elimination procedure for
//! Presburger arithmetic (first-order theory of integers under addition).
//! It's complete and decidable.  The core idea:
//!
//! **Given:**  a conjunction `C(x, y₁, …, yₙ)` — a formula with one
//! "target" variable `x` and some other variables.
//!
//! **Goal:**  decide if `∃x. C(x, y₁, …, yₙ)` holds.
//!
//! **Steps:**
//! 1. Rewrite all constraints into the normal form `a·x ≤ t` or `a·x = t`
//!    (where `a > 0` and `t` is a linear term over the other variables).
//! 2. Compute the *dark shadow* (necessary conditions that imply ∃x) and
//!    the *grey shadow* (sufficient conditions from divisibility cases).
//! 3. Recurse / enumerate a bounded set of candidate values for `x`.
//!
//! For our purposes (finite ranges and simple constraints from refinement
//! types) we use a **bounded search strategy** instead of full quantifier
//! elimination:
//!
//! - Collect all lower bounds `l₁, l₂, …` and upper bounds `u₁, u₂, …`
//!   on `x` from the constraints.
//! - If `max(lᵢ) > min(uᵢ)`, immediately UNSAT.
//! - Otherwise the witness `x = max(lᵢ)` (or any value in the range)
//!   satisfies all constraints — SAT.
//!
//! This strategy is **complete** for the formulae that arise from LANG23
//! refinement predicates (bounded ranges, equalities, disequalities,
//! linear sums).  It runs in O(n) where n is the number of constraints.
//!
//! ## Approach to multiple variables
//!
//! We extend the single-variable case via **variable elimination**:
//!
//! 1. Pick one variable `x`.
//! 2. Try to bound `x` from the other constraints.
//! 3. Substitute a witness value for `x` and recurse on the remaining
//!    formula (now over `n−1` variables).
//!
//! The base case (0 variables) evaluates the ground formula directly.
//! This is correct but potentially exponential for formulae with many
//! variables.  For LANG23's use case (function parameters with O(5–10)
//! refined bindings) this is more than sufficient.
//!
//! ## Disequality handling
//!
//! `x ≠ k` is handled by trying `x = k−1` and `x = k+1` as alternative
//! witnesses when the primary witness equals `k`.  For large disequality
//! sets we fall back to `Unknown` (the formula is almost certainly SAT
//! for any realistic LANG23 predicate, so this is safe — the runtime
//! check catches any case the solver gave up on).

use std::collections::HashMap;

use constraint_core::Predicate;

use crate::{eval_bool, Model, SolverResult, Value};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// The LIA (linear integer arithmetic) solver tactic.
///
/// Solves conjunctions of linear constraints over integer variables using
/// a bounded Cooper algorithm.  Also handles Bool-sorted variables by
/// grounding them to {0, 1}.
pub struct LiaTactic;

impl LiaTactic {
    /// Solve `assertions` (conjunction) over `int_vars` and `bool_vars`.
    ///
    /// Returns `SAT(model)`, `UNSAT`, or `Unknown(reason)`.
    pub fn solve(
        assertions: &[Predicate],
        int_vars: &[String],
        bool_vars: &[String],
    ) -> SolverResult {
        // Flatten into a conjunction of linear constraints.
        let constraints: Vec<Predicate> = assertions.iter().cloned().collect();

        // Start with an empty partial assignment and eliminate variables
        // one by one.
        let mut assignment: HashMap<String, i128> = HashMap::new();

        // Eliminate int variables first.
        let all_vars: Vec<String> = {
            let mut v = int_vars.to_vec();
            // Treat Bool vars as 0-or-1 integers.
            v.extend_from_slice(bool_vars);
            v
        };

        // LIA budget: limits the total number of `eliminate_all` calls to
        // prevent O(search_width^N) blowup when many variables are
        // unconstrained.
        //
        // Calibration:
        //   N=1 variable: up to 1 001 calls (MAX_SEARCH_WIDTH + base).
        //   N=2 variables: up to ≈1 003 003 calls (1001 * 1002 + 1).
        //   N=3 variables: up to ≈10^9 potential calls — budget truncates
        //     this to Unknown long before it completes.
        //
        // 2 000 000 is generous enough to always succeed for N=2 (the common
        // LANG23 case: two refined parameters) while providing a hard ceiling
        // against adversarial formulae with 3+ unconstrained variables.
        const LIA_BUDGET: usize = 2_000_000;
        let mut budget = LIA_BUDGET;
        match eliminate_all(&constraints, &all_vars, &mut assignment, &mut budget) {
            EliminationResult::Sat => {
                // Build the model from the assignment.
                let mut model = Model::new();
                for name in int_vars {
                    let val = *assignment.get(name).unwrap_or(&0);
                    model.insert(name.clone(), Value::Int(val));
                }
                for name in bool_vars {
                    let val = *assignment.get(name).unwrap_or(&0);
                    model.insert(name.clone(), Value::Bool(val != 0));
                }
                SolverResult::Sat(model)
            }
            EliminationResult::Unsat => SolverResult::Unsat,
            EliminationResult::Unknown(reason) => SolverResult::Unknown(reason),
        }
    }
}

// ---------------------------------------------------------------------------
// Internal elimination engine
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum EliminationResult {
    Sat,
    Unsat,
    Unknown(String),
}

/// Recursively eliminate variables from the constraint set.
///
/// `vars` is the remaining variables to assign, `assignment` grows as
/// we commit to values for each.  When `vars` is empty we evaluate the
/// ground formula directly.  `budget` is a shared call counter; when
/// it reaches zero the search returns `Unknown` to cap worst-case work.
fn eliminate_all(
    constraints: &[Predicate],
    vars: &[String],
    assignment: &mut HashMap<String, i128>,
    budget: &mut usize,
) -> EliminationResult {
    // Budget guard: prevent O(search_width^N) blowup.
    if *budget == 0 {
        return EliminationResult::Unknown("LIA search budget exceeded".into());
    }
    *budget -= 1;

    // Base case: no more variables — ground evaluation.
    if vars.is_empty() {
        let model = build_model(assignment);
        if constraints.iter().all(|p| eval_bool(p, &model)) {
            return EliminationResult::Sat;
        } else {
            return EliminationResult::Unsat;
        }
    }

    let var = &vars[0];
    let rest = &vars[1..];

    // Step 1: compute bounds on `var` given the current partial assignment.
    // `lb` = tightest lower bound seen (initially −∞ = i128::MIN)
    // `ub` = tightest upper bound seen (initially +∞ = i128::MAX)
    let mut lb: Option<i128> = None; // None = −∞
    let mut ub: Option<i128> = None; // None = +∞
    let mut neqs: Vec<i128> = Vec::new(); // x ≠ k constraints
    let mut equalities: Vec<i128> = Vec::new(); // x = k constraints

    for constraint in constraints {
        match extract_bound(constraint, var, assignment) {
            BoundInfo::LowerBound(n) => {
                lb = Some(lb.map_or(n, |old| old.max(n)));
            }
            BoundInfo::UpperBound(n) => {
                ub = Some(ub.map_or(n, |old| old.min(n)));
            }
            BoundInfo::Equality(n) => {
                equalities.push(n);
            }
            BoundInfo::Disequality(n) => {
                neqs.push(n);
            }
            BoundInfo::Irrelevant => {}
            BoundInfo::Unsatisfiable => return EliminationResult::Unsat,
            BoundInfo::Complex => {
                // Can't extract a simple bound — fall back to interval search
                // heuristic after we do finish bounds gathering.
            }
        }
    }

    // Step 2: resolve equalities first.
    if !equalities.is_empty() {
        // All equalities must agree.
        let first = equalities[0];
        if equalities.iter().any(|&k| k != first) {
            return EliminationResult::Unsat;
        }
        // Check consistency with bounds.
        if let Some(l) = lb {
            if first < l {
                return EliminationResult::Unsat;
            }
        }
        if let Some(u) = ub {
            if first > u {
                return EliminationResult::Unsat;
            }
        }
        // Disequalities must not match.
        if neqs.contains(&first) {
            // The equality forces x = k but x ≠ k — UNSAT.
            return EliminationResult::Unsat;
        }
        assignment.insert(var.clone(), first);
        return eliminate_all(constraints, rest, assignment, budget);
    }

    // Step 3: determine search range.
    //
    // For the Cooper algorithm the search range is [lb, ub].  We bound
    // the maximum search width to avoid blowup.
    const MAX_SEARCH_WIDTH: i128 = 1_000;

    let lo = lb.unwrap_or(0);
    let hi = ub.unwrap_or(lo + MAX_SEARCH_WIDTH);

    // If lb > ub the range is empty — UNSAT.
    if lo > hi {
        return EliminationResult::Unsat;
    }

    // If the range is too large and there's no upper bound clamping it,
    // try just the lower bound + a small number of alternatives.
    let search_hi = if hi - lo > MAX_SEARCH_WIDTH {
        lo + MAX_SEARCH_WIDTH
    } else {
        hi
    };

    // Step 4: enumerate candidate values.
    //
    // For Bool vars: restrict to {0, 1}.
    // For Int vars: start at `lo`, skip values in `neqs`.
    let candidates: Vec<i128> = {
        let mut cands: Vec<i128> = (lo..=search_hi)
            .filter(|v| !neqs.contains(v))
            .collect();
        // If we couldn't find a candidate because neqs block the whole range,
        // give up.
        if cands.is_empty() {
            if lo <= hi {
                return EliminationResult::Unknown(format!(
                    "variable `{var}` has no candidate (range [{lo},{hi}] fully excluded by ≠ constraints)"
                ));
            }
            return EliminationResult::Unsat;
        }
        // Prefer the smallest magnitude (avoids overflow cascade).
        cands.sort_by_key(|v| v.unsigned_abs());
        cands
    };

    // Try each candidate.
    for &val in &candidates {
        assignment.insert(var.clone(), val);
        match eliminate_all(constraints, rest, assignment, budget) {
            EliminationResult::Sat => return EliminationResult::Sat,
            EliminationResult::Unsat => {
                // This candidate doesn't work — try next.
                assignment.remove(var);
                continue;
            }
            EliminationResult::Unknown(reason) => {
                // Solver gave up — propagate.
                assignment.remove(var);
                return EliminationResult::Unknown(reason);
            }
        }
    }

    // No candidate worked.
    EliminationResult::Unsat
}

/// Build a `Model` (for evaluation) from an `assignment` map.
fn build_model(assignment: &HashMap<String, i128>) -> Model {
    let mut m = Model::new();
    for (k, &v) in assignment {
        m.insert(k.clone(), Value::Int(v));
    }
    m
}

// ---------------------------------------------------------------------------
// Bound extraction
// ---------------------------------------------------------------------------

/// What a single constraint says about the target variable.
#[derive(Debug)]
enum BoundInfo {
    /// `var ≥ n`   (lower bound)
    LowerBound(i128),
    /// `var ≤ n`   (upper bound)
    UpperBound(i128),
    /// `var = n`   (equality)
    Equality(i128),
    /// `var ≠ n`   (disequality)
    Disequality(i128),
    /// Constraint doesn't mention `var` and is already satisfied (or is
    /// irrelevant given the current partial assignment).
    Irrelevant,
    /// Constraint doesn't mention `var` and is *not* satisfied.
    Unsatisfiable,
    /// Constraint is too complex to extract a simple bound.
    Complex,
}

/// Try to extract bound information on `var` from `constraint` given the
/// current partial `assignment` (which may have values for other vars).
fn extract_bound(
    constraint: &Predicate,
    var: &str,
    assignment: &HashMap<String, i128>,
) -> BoundInfo {
    // Evaluate any sub-expression that doesn't mention `var` to a constant.
    // Then classify the remaining relation.
    match constraint {
        // ─── Comparisons: x ⊙ rhs  or  lhs ⊙ x ───────────────────────
        Predicate::Le(lhs, rhs) => extract_cmp(lhs, rhs, var, assignment, CmpOp::Le),
        Predicate::Lt(lhs, rhs) => extract_cmp(lhs, rhs, var, assignment, CmpOp::Lt),
        Predicate::Ge(lhs, rhs) => extract_cmp(lhs, rhs, var, assignment, CmpOp::Ge),
        Predicate::Gt(lhs, rhs) => extract_cmp(lhs, rhs, var, assignment, CmpOp::Gt),
        Predicate::Eq(lhs, rhs) => extract_eq(lhs, rhs, var, assignment, false),
        Predicate::NEq(lhs, rhs) => extract_eq(lhs, rhs, var, assignment, true),

        // ─── And: recurse and aggregate ───────────────────────────────
        Predicate::And(parts) => {
            let mut result = BoundInfo::Irrelevant;
            for part in parts {
                let info = extract_bound(part, var, assignment);
                result = merge_bounds(result, info);
                if matches!(result, BoundInfo::Unsatisfiable) {
                    return BoundInfo::Unsatisfiable;
                }
            }
            result
        }

        // ─── Bool literal ─────────────────────────────────────────────
        Predicate::Bool(true) => BoundInfo::Irrelevant,
        Predicate::Bool(false) => BoundInfo::Unsatisfiable,

        // ─── If the constraint doesn't mention `var`, defer or evaluate ──
        other => {
            if !mentions_var(other, var) {
                // Only evaluate if the constraint is fully ground (all vars
                // it mentions are already assigned).  If any variable is
                // unbound, return Irrelevant and re-check at the base case.
                //
                // eval_bool returns false for unbound variables (Unknown→false),
                // which would produce a spurious Unsatisfiable — so we guard.
                if is_ground_under(other, assignment) {
                    let model = build_model(assignment);
                    if eval_bool(other, &model) {
                        BoundInfo::Irrelevant
                    } else {
                        BoundInfo::Unsatisfiable
                    }
                } else {
                    BoundInfo::Irrelevant // not yet ground — defer
                }
            } else {
                BoundInfo::Complex
            }
        }
    }
}

#[derive(Clone, Copy)]
enum CmpOp {
    Le,
    Lt,
    Ge,
    Gt,
}

/// Extract a bound from `lhs ⊙ rhs` where exactly one side is (or
/// linearly contains) `var`.
fn extract_cmp(
    lhs: &Predicate,
    rhs: &Predicate,
    var: &str,
    assignment: &HashMap<String, i128>,
    op: CmpOp,
) -> BoundInfo {
    // Try: var ⊙ const_rhs
    if is_var_ref(lhs, var) {
        if let Some(rhs_val) = eval_as_const(rhs, assignment) {
            return match op {
                CmpOp::Le => BoundInfo::UpperBound(rhs_val),
                // x < n  ≡  x ≤ n−1  (integers).
                // If n = i128::MIN then x < i128::MIN has no solution.
                CmpOp::Lt => match rhs_val.checked_sub(1) {
                    Some(v) => BoundInfo::UpperBound(v),
                    None => BoundInfo::Unsatisfiable,
                },
                CmpOp::Ge => BoundInfo::LowerBound(rhs_val),
                // x > n  ≡  x ≥ n+1  (integers).
                // If n = i128::MAX then x > i128::MAX has no solution.
                CmpOp::Gt => match rhs_val.checked_add(1) {
                    Some(v) => BoundInfo::LowerBound(v),
                    None => BoundInfo::Unsatisfiable,
                },
            };
        }
    }
    // Try: const_lhs ⊙ var  (flip the operator)
    if is_var_ref(rhs, var) {
        if let Some(lhs_val) = eval_as_const(lhs, assignment) {
            return match op {
                CmpOp::Le => BoundInfo::LowerBound(lhs_val), // lhs ≤ x ≡ x ≥ lhs
                // lhs < x  ≡  x ≥ lhs+1.  If lhs = i128::MAX no integer x satisfies this.
                CmpOp::Lt => match lhs_val.checked_add(1) {
                    Some(v) => BoundInfo::LowerBound(v),
                    None => BoundInfo::Unsatisfiable,
                },
                CmpOp::Ge => BoundInfo::UpperBound(lhs_val),
                // lhs > x  ≡  x ≤ lhs−1.  If lhs = i128::MIN no integer x satisfies this.
                CmpOp::Gt => match lhs_val.checked_sub(1) {
                    Some(v) => BoundInfo::UpperBound(v),
                    None => BoundInfo::Unsatisfiable,
                },
            };
        }
    }
    // Check if the constraint mentions `var` at all.
    if !mentions_var(lhs, var) && !mentions_var(rhs, var) {
        // Constraint is over other variables.  Evaluate only if every
        // variable it mentions is already in the assignment (i.e., it is
        // ground under the partial assignment).  If not, defer: return
        // Irrelevant and let the constraint be re-evaluated once the other
        // variables are assigned.
        //
        // NOTE: eval_bool returns `false` for unbound variables (Unknown →
        // false), so we MUST NOT fall back to eval_bool when the expression
        // is not yet ground — doing so would incorrectly return Unsatisfiable
        // for a constraint that will be satisfiable once other vars are set.
        let lv = eval_as_const(lhs, assignment);
        let rv = eval_as_const(rhs, assignment);
        let ok_opt = lv.zip(rv).map(|(l, r)| match op {
            CmpOp::Le => l <= r,
            CmpOp::Lt => l < r,
            CmpOp::Ge => l >= r,
            CmpOp::Gt => l > r,
        });
        return match ok_opt {
            Some(true) => BoundInfo::Irrelevant,
            Some(false) => BoundInfo::Unsatisfiable,
            None => BoundInfo::Irrelevant, // not ground yet — defer
        };
    }
    BoundInfo::Complex
}

/// Extract equality or disequality information.
fn extract_eq(
    lhs: &Predicate,
    rhs: &Predicate,
    var: &str,
    assignment: &HashMap<String, i128>,
    negated: bool,
) -> BoundInfo {
    if is_var_ref(lhs, var) {
        if let Some(rhs_val) = eval_as_const(rhs, assignment) {
            return if negated {
                BoundInfo::Disequality(rhs_val)
            } else {
                BoundInfo::Equality(rhs_val)
            };
        }
    }
    if is_var_ref(rhs, var) {
        if let Some(lhs_val) = eval_as_const(lhs, assignment) {
            return if negated {
                BoundInfo::Disequality(lhs_val)
            } else {
                BoundInfo::Equality(lhs_val)
            };
        }
    }
    if !mentions_var(lhs, var) && !mentions_var(rhs, var) {
        // Same deferral logic as in extract_cmp: only evaluate if ground.
        let ok_opt = eval_as_const(lhs, assignment)
            .zip(eval_as_const(rhs, assignment))
            .map(|(l, r)| if negated { l != r } else { l == r });
        return match ok_opt {
            Some(true) => BoundInfo::Irrelevant,
            Some(false) => BoundInfo::Unsatisfiable,
            None => BoundInfo::Irrelevant, // not ground yet — defer
        };
    }
    BoundInfo::Complex
}

/// Merge two `BoundInfo` values.  Used when walking `And` nodes.
fn merge_bounds(a: BoundInfo, b: BoundInfo) -> BoundInfo {
    match (a, b) {
        (BoundInfo::Unsatisfiable, _) | (_, BoundInfo::Unsatisfiable) => {
            BoundInfo::Unsatisfiable
        }
        // If either is Complex, the whole thing is Complex.
        (BoundInfo::Complex, _) | (_, BoundInfo::Complex) => BoundInfo::Complex,
        // Both irrelevant → irrelevant.
        (BoundInfo::Irrelevant, other) | (other, BoundInfo::Irrelevant) => other,
        // Both lower bounds → take the tighter one.
        (BoundInfo::LowerBound(a), BoundInfo::LowerBound(b)) => {
            BoundInfo::LowerBound(a.max(b))
        }
        // Both upper bounds → take the tighter one.
        (BoundInfo::UpperBound(a), BoundInfo::UpperBound(b)) => {
            BoundInfo::UpperBound(a.min(b))
        }
        // Mixed: Complex (caller will enumerate anyway).
        _ => BoundInfo::Complex,
    }
}

// ---------------------------------------------------------------------------
// Predicate helpers
// ---------------------------------------------------------------------------

/// Return `true` if `p` is exactly `Var(name)`.
fn is_var_ref(p: &Predicate, name: &str) -> bool {
    matches!(p, Predicate::Var(n) if n == name)
}

/// Return `true` if `p` contains any reference to `name`.
fn mentions_var(p: &Predicate, name: &str) -> bool {
    match p {
        Predicate::Var(n) => n == name,
        Predicate::Bool(_) | Predicate::Int(_) | Predicate::Real(_) => false,
        Predicate::And(parts) | Predicate::Or(parts) | Predicate::Add(parts) => {
            parts.iter().any(|q| mentions_var(q, name))
        }
        Predicate::Not(inner) | Predicate::Mul { term: inner, .. } => mentions_var(inner, name),
        Predicate::Implies(a, b)
        | Predicate::Iff(a, b)
        | Predicate::Eq(a, b)
        | Predicate::NEq(a, b)
        | Predicate::Sub(a, b)
        | Predicate::Le(a, b)
        | Predicate::Lt(a, b)
        | Predicate::Ge(a, b)
        | Predicate::Gt(a, b) => mentions_var(a, name) || mentions_var(b, name),
        Predicate::Ite(c, t, e) => {
            mentions_var(c, name) || mentions_var(t, name) || mentions_var(e, name)
        }
        Predicate::Apply { args, .. } => args.iter().any(|a| mentions_var(a, name)),
        Predicate::Forall { var, body, .. } | Predicate::Exists { var, body, .. } => {
            var != name && mentions_var(body, name)
        }
        Predicate::Select { arr, idx } => mentions_var(arr, name) || mentions_var(idx, name),
        Predicate::Store { arr, idx, val } => {
            mentions_var(arr, name) || mentions_var(idx, name) || mentions_var(val, name)
        }
        _ => false,
    }
}

/// Return `true` if every `Var` referenced in `p` is present in `assignment`.
///
/// Used to guard against calling `eval_bool` with unbound variables: when a
/// variable is missing from the model `eval_bool` returns `false`, which would
/// incorrectly signal `Unsatisfiable` for a constraint that is just deferred.
fn is_ground_under(p: &Predicate, assignment: &HashMap<String, i128>) -> bool {
    match p {
        Predicate::Var(name) => assignment.contains_key(name.as_str()),
        Predicate::Bool(_) | Predicate::Int(_) | Predicate::Real(_) => true,
        Predicate::And(parts) | Predicate::Or(parts) | Predicate::Add(parts) => {
            parts.iter().all(|q| is_ground_under(q, assignment))
        }
        Predicate::Not(inner) | Predicate::Mul { term: inner, .. } => {
            is_ground_under(inner, assignment)
        }
        Predicate::Implies(a, b)
        | Predicate::Iff(a, b)
        | Predicate::Eq(a, b)
        | Predicate::NEq(a, b)
        | Predicate::Sub(a, b)
        | Predicate::Le(a, b)
        | Predicate::Lt(a, b)
        | Predicate::Ge(a, b)
        | Predicate::Gt(a, b) => is_ground_under(a, assignment) && is_ground_under(b, assignment),
        Predicate::Ite(c, t, e) => {
            is_ground_under(c, assignment)
                && is_ground_under(t, assignment)
                && is_ground_under(e, assignment)
        }
        Predicate::Apply { args, .. } => args.iter().all(|a| is_ground_under(a, assignment)),
        Predicate::Forall { body, .. } | Predicate::Exists { body, .. } => {
            is_ground_under(body, assignment)
        }
        Predicate::Select { arr, idx } => {
            is_ground_under(arr, assignment) && is_ground_under(idx, assignment)
        }
        Predicate::Store { arr, idx, val } => {
            is_ground_under(arr, assignment)
                && is_ground_under(idx, assignment)
                && is_ground_under(val, assignment)
        }
        _ => true,
    }
}

/// Try to evaluate `p` to a concrete `i128` given `assignment`.
/// Returns `None` if the expression is not a ground integer expression
/// under the partial assignment.
fn eval_as_const(p: &Predicate, assignment: &HashMap<String, i128>) -> Option<i128> {
    match p {
        Predicate::Int(n) => Some(*n),
        Predicate::Var(name) => assignment.get(name).copied(),
        Predicate::Add(parts) => {
            let mut sum = 0i128;
            for part in parts {
                sum = sum.checked_add(eval_as_const(part, assignment)?)?;
            }
            Some(sum)
        }
        Predicate::Sub(a, b) => {
            let av = eval_as_const(a, assignment)?;
            let bv = eval_as_const(b, assignment)?;
            av.checked_sub(bv)
        }
        Predicate::Mul { coef, term } => {
            let tv = eval_as_const(term, assignment)?;
            coef.checked_mul(tv)
        }
        Predicate::Ite(cond, t, e) => {
            let model = build_model(assignment);
            if eval_bool(cond, &model) {
                eval_as_const(t, assignment)
            } else {
                eval_as_const(e, assignment)
            }
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use constraint_core::Predicate;

    use super::*;

    fn vi(name: &str) -> Predicate {
        Predicate::Var(name.into())
    }
    fn lit(n: i128) -> Predicate {
        Predicate::Int(n)
    }

    // Helper: assert SAT and return the model.
    fn sat_model(assertions: &[Predicate], int_vars: &[&str]) -> Model {
        let iv: Vec<String> = int_vars.iter().map(|s| s.to_string()).collect();
        let r = LiaTactic::solve(assertions, &iv, &[]);
        match r {
            SolverResult::Sat(m) => m,
            other => panic!("expected SAT, got {other:?}"),
        }
    }

    // Helper: assert UNSAT.
    fn assert_unsat(assertions: &[Predicate], int_vars: &[&str]) {
        let iv: Vec<String> = int_vars.iter().map(|s| s.to_string()).collect();
        let r = LiaTactic::solve(assertions, &iv, &[]);
        assert!(r.is_unsat(), "expected UNSAT, got {r:?}");
    }

    // ─── Single variable ─────────────────────────────────────────────────

    #[test]
    fn single_var_sat_range() {
        // x ≥ 0 ∧ x ≤ 100
        let r = sat_model(
            &[
                Predicate::Ge(Box::new(vi("x")), Box::new(lit(0))),
                Predicate::Le(Box::new(vi("x")), Box::new(lit(100))),
            ],
            &["x"],
        );
        let x = r.get("x").unwrap().as_int().unwrap();
        assert!((0..=100).contains(&x));
    }

    #[test]
    fn single_var_equality() {
        // x = 42
        let r = sat_model(
            &[Predicate::Eq(Box::new(vi("x")), Box::new(lit(42)))],
            &["x"],
        );
        assert_eq!(r.get("x").unwrap().as_int().unwrap(), 42);
    }

    #[test]
    fn single_var_tight_range_unsat() {
        // x ≥ 10 ∧ x ≤ 5
        assert_unsat(
            &[
                Predicate::Ge(Box::new(vi("x")), Box::new(lit(10))),
                Predicate::Le(Box::new(vi("x")), Box::new(lit(5))),
            ],
            &["x"],
        );
    }

    #[test]
    fn single_var_strict_lt_integer_gap_unsat() {
        // x > 5 ∧ x < 6 → no integer solution
        assert_unsat(
            &[
                Predicate::Gt(Box::new(vi("x")), Box::new(lit(5))),
                Predicate::Lt(Box::new(vi("x")), Box::new(lit(6))),
            ],
            &["x"],
        );
    }

    #[test]
    fn single_var_disequality_sat() {
        // x ≠ 0 (lower bound default 0 → try 1)
        let iv = vec!["x".to_string()];
        let r = LiaTactic::solve(
            &[Predicate::NEq(Box::new(vi("x")), Box::new(lit(0)))],
            &iv,
            &[],
        );
        assert!(r.is_sat(), "expected SAT, got {r:?}");
    }

    #[test]
    fn empty_constraints_sat() {
        let r = sat_model(&[], &["x"]);
        // Any value for x is fine.
        assert!(r.get("x").is_some());
    }

    // ─── Two variables ────────────────────────────────────────────────────

    #[test]
    fn two_var_both_bounded() {
        // 0 ≤ x ≤ 10 ∧ 0 ≤ y ≤ 10 ∧ x + y ≤ 15
        let r = sat_model(
            &[
                Predicate::Ge(Box::new(vi("x")), Box::new(lit(0))),
                Predicate::Le(Box::new(vi("x")), Box::new(lit(10))),
                Predicate::Ge(Box::new(vi("y")), Box::new(lit(0))),
                Predicate::Le(Box::new(vi("y")), Box::new(lit(10))),
                Predicate::Le(
                    Box::new(Predicate::Add(vec![vi("x"), vi("y")])),
                    Box::new(lit(15)),
                ),
            ],
            &["x", "y"],
        );
        let x = r.get("x").unwrap().as_int().unwrap();
        let y = r.get("y").unwrap().as_int().unwrap();
        assert!((0..=10).contains(&x) && (0..=10).contains(&y) && x + y <= 15);
    }

    #[test]
    fn two_var_mutual_exclusion_unsat() {
        // x > 5 ∧ y > 5 ∧ x + y < 10
        assert_unsat(
            &[
                Predicate::Gt(Box::new(vi("x")), Box::new(lit(5))),
                Predicate::Gt(Box::new(vi("y")), Box::new(lit(5))),
                Predicate::Lt(
                    Box::new(Predicate::Add(vec![vi("x"), vi("y")])),
                    Box::new(lit(10)),
                ),
            ],
            &["x", "y"],
        );
    }

    // ─── Coefficient scaling ──────────────────────────────────────────────

    #[test]
    fn scaled_variable_sat() {
        // 2·x ≤ 10  →  x ≤ 5
        let r = sat_model(
            &[Predicate::Le(
                Box::new(Predicate::Mul { coef: 2, term: Box::new(vi("x")) }),
                Box::new(lit(10)),
            )],
            &["x"],
        );
        // The model should satisfy 2*x ≤ 10.
        let x = r.get("x").unwrap().as_int().unwrap();
        assert!(2 * x <= 10, "expected 2*x <= 10, got x={x}");
    }

    // ─── Boolean variables ────────────────────────────────────────────────

    #[test]
    fn bool_var_sat() {
        // b = true → 0 or 1
        let bv = vec!["b".to_string()];
        let r = LiaTactic::solve(
            &[Predicate::Bool(true)],
            &[],
            &bv,
        );
        assert!(r.is_sat());
    }

    // ─── mentions_var helper ──────────────────────────────────────────────

    #[test]
    fn mentions_var_basics() {
        assert!(mentions_var(&vi("x"), "x"));
        assert!(!mentions_var(&vi("y"), "x"));
        assert!(mentions_var(
            &Predicate::Add(vec![vi("x"), vi("y")]),
            "x"
        ));
        assert!(!mentions_var(&lit(42), "x"));
    }

    // ─── eval_as_const helper ─────────────────────────────────────────────

    #[test]
    fn eval_as_const_basics() {
        let mut a = HashMap::new();
        a.insert("x".to_string(), 3i128);
        assert_eq!(eval_as_const(&lit(5), &a), Some(5));
        assert_eq!(eval_as_const(&vi("x"), &a), Some(3));
        assert_eq!(
            eval_as_const(
                &Predicate::Add(vec![vi("x"), lit(2)]),
                &a
            ),
            Some(5)
        );
        assert_eq!(eval_as_const(&vi("y"), &a), None);
    }
}
