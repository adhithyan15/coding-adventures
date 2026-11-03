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

use std::collections::{HashMap, HashSet};

use constraint_core::Predicate;

use crate::{truth, Model, SolverResult, Value};

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
        // Flatten nested conjunctions into separate constraints, so that
        // `(and (>= x a) (<= x b))` contributes both of its bounds (one
        // `And` merges a lower and an upper bound into "complex").
        let mut constraints: Vec<Predicate> = Vec::new();
        let mut pending: Vec<&Predicate> = assertions.iter().rev().collect();
        while let Some(p) = pending.pop() {
            match p {
                Predicate::And(parts) => pending.extend(parts.iter().rev()),
                other => constraints.push(other.clone()),
            }
        }

        // Start with an empty partial assignment and eliminate variables
        // one by one.
        let mut assignment: HashMap<String, i128> = HashMap::new();

        // Eliminate int variables first. A variable no constraint mentions
        // can take any value: it is not searched (searching it unboxed would
        // only turn a proof into Unknown), and the model gives it 0 / false.
        let mentioned: std::collections::BTreeSet<String> =
            constraints.iter().flat_map(|c| c.free_vars()).collect();
        let all_vars: Vec<String> = {
            let mut v = int_vars.to_vec();
            // Treat Bool vars as 0-or-1 integers.
            v.extend_from_slice(bool_vars);
            v.retain(|name| mentioned.contains(name));
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
        // Tighten the search with bounds the constraints imply (interval
        // propagation over the linear atoms). Every derived bound is a
        // consequence of the constraints, so adding it changes no answer;
        // it only lets the search be exhaustive where it otherwise samples.
        let ints: HashSet<&str> = int_vars.iter().map(String::as_str).collect();
        match propagate_intervals(&constraints, &ints) {
            Propagated::Contradiction => return SolverResult::Unsat,
            Propagated::Bounds(bounds) => constraints.extend(bounds),
        }

        let mut budget = LIA_BUDGET;
        let bools: HashSet<String> = bool_vars.iter().cloned().collect();
        match eliminate_all(&constraints, &all_vars, &bools, &mut assignment, &mut budget) {
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
    bools: &HashSet<String>,
    assignment: &mut HashMap<String, i128>,
    budget: &mut usize,
) -> EliminationResult {
    // Budget guard: prevent O(search_width^N) blowup.
    if *budget == 0 {
        return EliminationResult::Unknown("LIA search budget exceeded".into());
    }
    *budget -= 1;

    // Base case: no more variables — ground evaluation, three-valued. A
    // constraint that cannot be evaluated (overflow, an unsupported term)
    // makes the whole answer Unknown: concluding UNSAT from "could not
    // evaluate" would be a false proof.
    if vars.is_empty() {
        let model = build_model(assignment);
        let mut unknown = false;
        for p in constraints {
            match truth(p, &model) {
                Some(true) => {}
                Some(false) => return EliminationResult::Unsat,
                None => unknown = true,
            }
        }
        return if unknown {
            EliminationResult::Unknown("a constraint could not be evaluated (overflow or unsupported term)".into())
        } else {
            EliminationResult::Sat
        };
    }

    let var = &vars[0];
    let rest = &vars[1..];

    // Step 1: compute bounds on `var` given the current partial assignment.
    // `lb` = tightest lower bound seen (initially −∞ = i128::MIN)
    // `ub` = tightest upper bound seen (initially +∞ = i128::MAX)
    let mut lb: Option<i128> = None; // None = −∞
    let mut ub: Option<i128> = None; // None = +∞
    // HashSet for O(1) membership test in the candidate-filter below.
    let mut neqs: HashSet<i128> = HashSet::new(); // x ≠ k constraints
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
                neqs.insert(n);
            }
            BoundInfo::Irrelevant => {}
            BoundInfo::Unsatisfiable => return EliminationResult::Unsat,
            BoundInfo::Unknown => {
                return EliminationResult::Unknown(
                    "a ground constraint could not be evaluated (overflow or unsupported term)".into(),
                )
            }
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
        let result = eliminate_all(constraints, rest, bools, assignment, budget);
        // Leave the assignment as it was found unless this is the model.
        // A value left behind after a failed branch would make a later
        // branch judge constraints "ground" under a stale value: a false
        // UNSAT.
        if !matches!(result, EliminationResult::Sat) {
            assignment.remove(var);
        }
        return result;
    }

    // Step 3: decide which values to try, and whether trying them all is a
    // proof (LANG24 "LIA as built, and its soundness contract").
    //
    // The search is exhaustive only when every value the variable could
    // take is tried: a Bool (domain {0, 1}), or an Int boxed in by a real
    // lower and a real upper bound at most MAX_SEARCH_WIDTH apart. Anything
    // else is a *sample*. A sample can find a model (SAT), but its failing
    // proves nothing: the answer is then Unknown, never UNSAT.
    //
    //   bounds              values tried                       exhaustive?
    //   Bool                {0, 1} ∩ [lb, ub]                  yes
    //   lb and ub, narrow   [lb, ub]                           yes
    //   lb and ub, wide     [lb, lb + W]                       no
    //   lb only             [lb, lb + W]                       no
    //   ub only             [ub − W, ub]                       no
    //   neither             [−W/2, W/2]                        no
    const MAX_SEARCH_WIDTH: i128 = 1_000;

    let is_bool = bools.contains(var);
    if let (Some(l), Some(u)) = (lb, ub) {
        if l > u {
            // Two real bounds that contradict: no integer fits. A proof.
            return EliminationResult::Unsat;
        }
    }
    let (lo, hi, exhaustive) = if is_bool {
        (lb.unwrap_or(0).max(0), ub.unwrap_or(1).min(1), true)
    } else {
        match (lb, ub) {
            (Some(l), Some(u)) => match u.checked_sub(l) {
                Some(width) if width <= MAX_SEARCH_WIDTH => (l, u, true),
                _ => (l, l.saturating_add(MAX_SEARCH_WIDTH), false),
            },
            (Some(l), None) => (l, l.saturating_add(MAX_SEARCH_WIDTH), false),
            (None, Some(u)) => (u.saturating_sub(MAX_SEARCH_WIDTH), u, false),
            (None, None) => (-MAX_SEARCH_WIDTH / 2, MAX_SEARCH_WIDTH / 2, false),
        }
    };

    // Step 4: enumerate the candidates, smallest magnitude first.
    let mut candidates: Vec<i128> = (lo..=hi).filter(|v| !neqs.contains(v)).collect();
    if candidates.is_empty() {
        // Every value in range is excluded by ≠ (or the Bool domain is cut
        // empty by its bounds). That is a proof only if the range was
        // everything the variable could be.
        return if exhaustive {
            EliminationResult::Unsat
        } else {
            EliminationResult::Unknown(format!(
                "variable `{var}`: every sampled value in [{lo}, {hi}] is excluded by ≠ constraints"
            ))
        };
    }
    candidates.sort_by_key(|v| v.unsigned_abs());

    // Try each candidate. A child's UNSAT only means "this value fails"; a
    // child's Unknown is remembered, and the search goes on, since a later
    // value may still give a model.
    let mut unknown: Option<String> = None;
    for &val in &candidates {
        assignment.insert(var.clone(), val);
        let result = eliminate_all(constraints, rest, bools, assignment, budget);
        assignment.remove(var);
        match result {
            EliminationResult::Sat => {
                assignment.insert(var.clone(), val);
                return EliminationResult::Sat;
            }
            EliminationResult::Unsat => {}
            EliminationResult::Unknown(reason) => {
                unknown.get_or_insert(reason);
            }
        }
    }

    // No candidate worked.
    match unknown {
        Some(reason) => EliminationResult::Unknown(reason),
        None if exhaustive => EliminationResult::Unsat,
        None => EliminationResult::Unknown(format!(
            "variable `{var}`: no model among the sampled values [{lo}, {hi}], and the search \
             there is not exhaustive (missing or distant bounds)"
        )),
    }
}

// ---------------------------------------------------------------------------
// Interval propagation
// ---------------------------------------------------------------------------
//
// The enumeration below can only prove UNSAT for a variable it can box in
// (LANG24 soundness contract). Constraints often box a variable only
// indirectly: in `x > 5 ∧ y > 5 ∧ x + y < 10`, `x` has no upper bound of its
// own, but `x + y < 10` with `y ≥ 6` gives `x ≤ 3`, and that contradicts
// `x ≥ 6`. This pass derives such bounds.
//
// Every top-level linear atom is normalised to `Σ aᵢ·xᵢ + c ≤ 0`. For each
// variable `xⱼ` in it, the other terms are at least their interval minimum
// `S`, so `aⱼ·xⱼ ≤ −c − S`, which bounds `xⱼ` from above (aⱼ > 0) or below
// (aⱼ < 0). Rounds repeat until nothing tightens, up to a cap: a cyclic
// system like `x < y ∧ y < x` tightens by one per round forever, and
// stopping early is always sound, only less precise.

/// `Σ terms[v]·v + constant`, over integer variables.
struct Linear {
    terms: HashMap<String, i128>,
    constant: i128,
}

/// The linear form of an integer expression, or `None` if it is not linear
/// over `ints` or its coefficients overflow.
fn linearize(p: &Predicate, ints: &HashSet<&str>) -> Option<Linear> {
    match p {
        Predicate::Int(n) => Some(Linear { terms: HashMap::new(), constant: *n }),
        Predicate::Var(v) if ints.contains(v.as_str()) => {
            Some(Linear { terms: HashMap::from([(v.clone(), 1)]), constant: 0 })
        }
        Predicate::Add(parts) => {
            let mut acc = Linear { terms: HashMap::new(), constant: 0 };
            for q in parts {
                acc = combine(acc, linearize(q, ints)?, 1)?;
            }
            Some(acc)
        }
        Predicate::Sub(a, b) => combine(linearize(a, ints)?, linearize(b, ints)?, -1),
        Predicate::Mul { coef, term } => {
            let t = linearize(term, ints)?;
            let mut terms = HashMap::new();
            for (v, c) in t.terms {
                terms.insert(v, c.checked_mul(*coef)?);
            }
            Some(Linear { terms, constant: t.constant.checked_mul(*coef)? })
        }
        _ => None,
    }
}

/// `a + sign·b`, with overflow checks.
fn combine(mut a: Linear, b: Linear, sign: i128) -> Option<Linear> {
    for (v, c) in b.terms {
        let e = a.terms.entry(v).or_insert(0);
        *e = e.checked_add(c.checked_mul(sign)?)?;
    }
    a.constant = a.constant.checked_add(b.constant.checked_mul(sign)?)?;
    Some(a)
}

/// The atoms of `p`, normalised to `form ≤ 0`. Non-linear or unsupported
/// constraints contribute nothing (they are still checked by the search).
fn atoms_le_zero(p: &Predicate, ints: &HashSet<&str>) -> Vec<Linear> {
    let diff = |a: &Predicate, b: &Predicate, plus: i128| -> Option<Linear> {
        let mut d = combine(linearize(a, ints)?, linearize(b, ints)?, -1)?;
        d.constant = d.constant.checked_add(plus)?;
        Some(d)
    };
    let out = match p {
        Predicate::Le(a, b) => vec![diff(a, b, 0)],
        Predicate::Lt(a, b) => vec![diff(a, b, 1)], // a < b ⇔ a − b + 1 ≤ 0
        Predicate::Ge(a, b) => vec![diff(b, a, 0)],
        Predicate::Gt(a, b) => vec![diff(b, a, 1)],
        Predicate::Eq(a, b) => vec![diff(a, b, 0), diff(b, a, 0)],
        // A negated comparison is the opposite comparison (integers are
        // totally ordered): ¬(a ≤ b) ⇔ a > b, and so on. (¬(a = b) bounds
        // nothing.)
        Predicate::Not(inner) => match inner.as_ref() {
            Predicate::Le(a, b) => vec![diff(b, a, 1)],
            Predicate::Lt(a, b) => vec![diff(b, a, 0)],
            Predicate::Ge(a, b) => vec![diff(a, b, 1)],
            Predicate::Gt(a, b) => vec![diff(a, b, 0)],
            _ => vec![],
        },
        _ => vec![],
    };
    out.into_iter().flatten().collect()
}

/// `⌊a / b⌋` and `⌈a / b⌉` for `b ≠ 0`, or `None` on overflow.
fn floor_div(a: i128, b: i128) -> Option<i128> {
    let q = a.checked_div(b)?;
    if (a % b != 0) && ((a < 0) != (b < 0)) { q.checked_sub(1) } else { Some(q) }
}

fn ceil_div(a: i128, b: i128) -> Option<i128> {
    let q = a.checked_div(b)?;
    if (a % b != 0) && ((a < 0) == (b < 0)) { q.checked_add(1) } else { Some(q) }
}

/// What interval propagation found.
enum Propagated {
    /// Implied bounds, as constraints (`x ≥ l`, `x ≤ u`).
    Bounds(Vec<Predicate>),
    /// Some variable's implied interval is empty: the constraints are UNSAT.
    Contradiction,
}

/// Derive the bounds the top-level linear atoms of `constraints` imply.
///
/// Per atom and round the work is linear in its terms: the minimum of the
/// whole sum is computed once (a finite part plus a count of unbounded
/// terms), and each variable's bound subtracts its own term back out. A
/// global cap on term visits bounds the total; stopping early is sound.
fn propagate_intervals(constraints: &[Predicate], ints: &HashSet<&str>) -> Propagated {
    const MAX_ROUNDS: usize = 64;
    const MAX_TERM_VISITS: usize = 2_000_000;
    let atoms: Vec<Linear> = constraints.iter().flat_map(|c| atoms_le_zero(c, ints)).collect();
    // Interval per variable: (lower, upper); None is unbounded.
    let mut iv: HashMap<String, (Option<i128>, Option<i128>)> = HashMap::new();
    let mut visits = 0usize;
    'rounds: for _ in 0..MAX_ROUNDS {
        let mut changed = false;
        for atom in &atoms {
            visits = visits.saturating_add(2 * atom.terms.len());
            if visits > MAX_TERM_VISITS {
                break 'rounds;
            }
            // min(aᵢ·xᵢ) for each term, or None when that term is unbounded
            // below (or overflows).
            let min_of = |x: &str, a: i128, iv: &HashMap<String, (Option<i128>, Option<i128>)>| {
                let (lo, hi) = iv.get(x).copied().unwrap_or((None, None));
                if a > 0 { lo.and_then(|l| a.checked_mul(l)) } else { hi.and_then(|h| a.checked_mul(h)) }
            };
            let mins: Vec<(&String, i128, Option<i128>)> =
                atom.terms.iter().filter(|(_, &a)| a != 0).map(|(x, &a)| (x, a, min_of(x, a, &iv))).collect();
            let unbounded = mins.iter().filter(|(_, _, m)| m.is_none()).count();
            if unbounded > 1 {
                continue; // every variable has another unbounded term beside it
            }
            // c + Σ of the finite minimums.
            let Some(finite) = mins
                .iter()
                .filter_map(|(_, _, m)| *m)
                .try_fold(atom.constant, |acc, m| acc.checked_add(m))
            else {
                continue;
            };
            for &(xj, aj, min_j) in &mins {
                // S = Σ_{i≠j} min(aᵢ·xᵢ) + c
                let s = match (unbounded, min_j) {
                    (0, Some(m)) => finite.checked_sub(m),
                    (1, None) => Some(finite), // xⱼ is the one unbounded term
                    _ => None,                 // another term is unbounded
                };
                // aⱼ·xⱼ ≤ −S
                let Some(rhs) = s.and_then(|v| v.checked_neg()) else { continue };
                let entry = iv.entry(xj.clone()).or_insert((None, None));
                if aj > 0 {
                    if let Some(u) = floor_div(rhs, aj) {
                        if entry.1.is_none_or(|old| u < old) {
                            entry.1 = Some(u);
                            changed = true;
                        }
                    }
                } else if let Some(l) = ceil_div(rhs, aj) {
                    if entry.0.is_none_or(|old| l > old) {
                        entry.0 = Some(l);
                        changed = true;
                    }
                }
                if let (Some(l), Some(u)) = *entry {
                    if l > u {
                        return Propagated::Contradiction;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    let mut bounds = Vec::new();
    let mut names: Vec<_> = iv.into_iter().collect();
    names.sort_by(|a, b| a.0.cmp(&b.0)); // deterministic order
    for (v, (lo, hi)) in names {
        let var = || Box::new(Predicate::Var(v.clone()));
        if let Some(l) = lo {
            bounds.push(Predicate::Ge(var(), Box::new(Predicate::Int(l))));
        }
        if let Some(u) = hi {
            bounds.push(Predicate::Le(var(), Box::new(Predicate::Int(u))));
        }
    }
    Propagated::Bounds(bounds)
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
    /// Constraint doesn't mention `var` and is ground, but cannot be
    /// evaluated (an overflow, an unsupported term). The search must answer
    /// Unknown rather than guess.
    Unknown,
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

        // (No `And` arm: `LiaTactic::solve` flattens conjunctions before the
        // search, so every constraint reaching here is a single conjunct.)

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
                    match truth(other, &model) {
                        Some(true) => BoundInfo::Irrelevant,
                        Some(false) => BoundInfo::Unsatisfiable,
                        None => BoundInfo::Unknown,
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
            // A condition that cannot be evaluated yet (unassigned
            // variables, overflow) means "not a constant yet": defer.
            if truth(cond, &model)? {
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

    // ─── Soundness contract (LANG24): every UNSAT is a proof ──────────────

    fn solve(assertions: &[Predicate], ints: &[&str]) -> SolverResult {
        let ints: Vec<String> = ints.iter().map(|s| s.to_string()).collect();
        LiaTactic::solve(assertions, &ints, &[])
    }

    fn cmp(f: fn(Box<Predicate>, Box<Predicate>) -> Predicate, v: &str, k: i128) -> Predicate {
        f(Box::new(vi(v)), Box::new(lit(k)))
    }

    #[test]
    fn equalities_that_conflict_are_proofs() {
        let eq = |k| cmp(Predicate::Eq, "x", k);
        assert_unsat(&[eq(1), eq(2)], &["x"]);
        assert_unsat(&[eq(5), cmp(Predicate::Ge, "x", 10)], &["x"]);
        assert_unsat(&[eq(5), cmp(Predicate::Le, "x", 1)], &["x"]);
        assert_unsat(&[eq(5), cmp(Predicate::NEq, "x", 5)], &["x"]);
        assert_unsat(&[cmp(Predicate::Ge, "x", 3), cmp(Predicate::Le, "x", 2)], &["x"]);
    }

    #[test]
    fn exclusions_prove_only_when_the_range_is_complete() {
        // x ∈ [0, 1], x ≠ 0, x ≠ 1: every value excluded, a proof.
        let boxed = [
            cmp(Predicate::Ge, "x", 0),
            cmp(Predicate::Le, "x", 1),
            cmp(Predicate::NEq, "x", 0),
            cmp(Predicate::NEq, "x", 1),
        ];
        assert_unsat(&boxed, &["x"]);
        // Unbounded x with the whole sample window excluded: Unknown.
        let window: Vec<Predicate> = (-500..=500).map(|k| cmp(Predicate::NEq, "x", k)).collect();
        assert!(matches!(solve(&window, &["x"]), SolverResult::Unknown(_)));
    }

    #[test]
    fn samples_that_fail_are_unknown_not_unsat() {
        // A wide box with the model behind a disjunction (which propagation
        // does not read): only [0, 1000] is sampled, the model 3000 lies
        // beyond.
        let a = [
            cmp(Predicate::Ge, "x", 0),
            cmp(Predicate::Le, "x", 5000),
            Predicate::Or(vec![cmp(Predicate::Eq, "x", 3000), cmp(Predicate::Eq, "x", 4000)]),
        ];
        assert!(matches!(solve(&a, &["x"]), SolverResult::Unknown(_)));
        // The same with a linear equality is decided exactly: propagation
        // turns 3x = 9000 into x ∈ [3000, 3000].
        let b = [
            cmp(Predicate::Ge, "x", 0),
            cmp(Predicate::Le, "x", 5000),
            Predicate::Eq(Box::new(Predicate::Mul { coef: 3, term: Box::new(vi("x")) }), Box::new(lit(9000))),
        ];
        assert_eq!(sat_model(&b, &["x"]).get("x").unwrap().as_int(), Some(3000));
        // Only an upper bound: the sample ends at it, so negatives are found.
        let r = solve(&[cmp(Predicate::Lt, "x", -3)], &["x"]);
        assert!(r.model().unwrap().get("x").unwrap().as_int().unwrap() < -3);
    }

    #[test]
    fn bounds_at_the_i128_edge_are_exact() {
        // x < MIN and x > MAX have no integer solutions; MIN − 1 does not exist.
        assert_unsat(&[cmp(Predicate::Lt, "x", i128::MIN)], &["x"]);
        assert_unsat(&[cmp(Predicate::Gt, "x", i128::MAX)], &["x"]);
        let flipped = |f: fn(Box<Predicate>, Box<Predicate>) -> Predicate, k| f(Box::new(lit(k)), Box::new(vi("x")));
        assert_unsat(&[flipped(Predicate::Lt, i128::MAX)], &["x"]);
        assert_unsat(&[flipped(Predicate::Gt, i128::MIN)], &["x"]);
    }

    #[test]
    fn what_cannot_be_evaluated_is_unknown() {
        // Called directly (no engine guard): an uninterpreted term.
        let f = |args: Vec<Predicate>| Predicate::Apply { f: "f".into(), args };
        let ground = Predicate::Eq(Box::new(f(vec![])), Box::new(lit(1)));
        assert!(matches!(solve(&[ground], &["x"]), SolverResult::Unknown(_)));
        let over_x = Predicate::Eq(Box::new(f(vec![vi("x")])), Box::new(lit(1)));
        assert!(matches!(solve(&[cmp(Predicate::Ge, "x", 0), cmp(Predicate::Le, "x", 2), over_x], &["x"]), SolverResult::Unknown(_)));
        // `true` constrains nothing.
        assert!(solve(&[Predicate::Bool(true), cmp(Predicate::Ge, "x", 4)], &["x"]).is_sat());
    }

    #[test]
    fn a_failed_equality_branch_leaves_no_stale_value() {
        // x ∈ [0,1], y ∈ [0,0], z = x + 10, x + z > 10. Model: x=1, y=0, z=11.
        // With the order x, y, z: x=0 → y=0 → z=10 (by equality) fails, and
        // z = 10 used to stay assigned, so for x=1 the constraint z = x + 10
        // looked ground and false while y was eliminated: a false UNSAT.
        let z_eq = Predicate::Eq(Box::new(vi("z")), Box::new(Predicate::Add(vec![vi("x"), lit(10)])));
        let sum_gt = Predicate::Gt(Box::new(Predicate::Add(vec![vi("x"), vi("z")])), Box::new(lit(10)));
        let a = [
            cmp(Predicate::Ge, "x", 0),
            cmp(Predicate::Le, "x", 1),
            cmp(Predicate::Ge, "y", 0),
            cmp(Predicate::Le, "y", 0),
            z_eq,
            sum_gt,
        ];
        let m = sat_model(&a, &["x", "y", "z"]);
        assert_eq!(m.get("z").unwrap().as_int(), Some(11));
    }

    #[test]
    fn propagation_on_wide_sums_is_cheap_and_capped() {
        // One 3000-term sum and a cycle that tightens a bound every round:
        // must finish quickly (linear per atom, capped overall) and stay
        // sound (the cycle x < y < x is UNSAT, but nothing here may claim a
        // wrong answer).
        let names: Vec<String> = (0..3000).map(|i| format!("v{i}")).collect();
        let sum = Predicate::Add(names.iter().map(|n| vi(n)).collect());
        let mut a = vec![Predicate::Le(Box::new(sum), Box::new(lit(0)))];
        a.push(cmp(Predicate::Ge, "a", 0));
        a.push(Predicate::Lt(Box::new(vi("a")), Box::new(vi("b"))));
        a.push(Predicate::Lt(Box::new(vi("b")), Box::new(vi("a"))));
        let ints: HashSet<&str> = names.iter().map(String::as_str).chain(["a", "b"]).collect();
        let start = std::time::Instant::now();
        let r = propagate_intervals(&a, &ints);
        assert!(start.elapsed().as_secs() < 10, "propagation took {:?}", start.elapsed());
        assert!(matches!(r, Propagated::Bounds(_) | Propagated::Contradiction));
    }

    #[test]
    fn propagation_helpers() {
        assert_eq!(floor_div(7, 2), Some(3));
        assert_eq!(floor_div(-7, 2), Some(-4));
        assert_eq!(floor_div(7, -2), Some(-4));
        assert_eq!(ceil_div(7, 2), Some(4));
        assert_eq!(ceil_div(-7, 2), Some(-3));
        assert_eq!(ceil_div(-7, -2), Some(4));
        assert_eq!(floor_div(i128::MIN, -1), None);
        let ints: HashSet<&str> = ["x", "y"].into_iter().collect();
        // Non-linear and unknown variables contribute nothing.
        assert!(linearize(&vi("z"), &ints).is_none());
        assert!(linearize(&Predicate::Mul { coef: i128::MAX, term: Box::new(lit(2)) }, &ints).is_none());
        assert!(atoms_le_zero(&Predicate::Or(vec![]), &ints).is_empty());
        assert!(atoms_le_zero(&Predicate::Not(Box::new(cmp(Predicate::Eq, "x", 1))), &ints).is_empty());
        assert_eq!(atoms_le_zero(&cmp(Predicate::Eq, "x", 1), &ints).len(), 2);
        // Negated comparisons are bounds: ¬(x ≤ 3) ∧ ¬(x > 5) boxes x to [4, 5].
        let r = solve(
            &[
                Predicate::Not(Box::new(cmp(Predicate::Le, "x", 3))),
                Predicate::Not(Box::new(cmp(Predicate::Gt, "x", 5))),
                cmp(Predicate::NEq, "x", 4),
                cmp(Predicate::NEq, "x", 5),
            ],
            &["x"],
        );
        assert!(r.is_unsat(), "{r:?}");
        // Coefficients other than ±1 round correctly: 2x ≤ 7 ∧ 2x ≥ 7 has no integer x.
        let two_x = || Box::new(Predicate::Mul { coef: 2, term: Box::new(vi("x")) });
        assert_unsat(&[Predicate::Le(two_x(), Box::new(lit(7))), Predicate::Ge(two_x(), Box::new(lit(7)))], &["x"]);
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
