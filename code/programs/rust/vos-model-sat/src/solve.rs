//! # `solve`: one budgeted solve, a DRAT check under the default limits, and a unit propagator
//!
//! VOS31 section 7.3 (decision Q3) says this lane does **not** depend on `vos-capacity-probe`: the
//! probe's `src` is a recorded input (section 3) and its library carries a scheduler harness this
//! lane should not link.  So the roughly 150 lines that the probe keeps in its `solve.rs` are
//! written again here, in the shape the probe uses (`solve_cnf`, `check_default`,
//! `model_satisfies`), minus the wall clock: an obligation is bounded by a **conflict count**, so
//! the answer is the same on every machine (the committed table never depends on a clock).
//!
//! ```text
//!   Cnf --solve_cnf--> Unsat + DRAT text --check_default--> Accepted { work }   (the UNSAT half)
//!   Cnf --solve_cnf--> Sat(model)        --model_satisfies--> true              (the SAT half)
//!   Cnf --propagation_refutes--> bool                                           (collapse, 7.5)
//! ```
//!
//! ## The route budget (VOS31 7.3)
//!
//! The budget is **the route budget of VOS30 8.6.1**: [`ROUTE_MAX_CONFLICTS`] conflicts and
//! [`ROUTE_MAX_CHECK_WORK`] literal visits of the checker.  The two constants are frozen by that
//! section; the test `route_budget_is_the_one_vos30_states` reads the section and fails if it
//! says anything else.  The checker always runs under `drat_check::Limits::default()` and the
//! limits are never raised (VOS30 6.3): a proof the default limits reject is an obligation that
//! is too big, and the remedy is a better encoding.

use cdcl_sat::{write_dimacs, Cnf, Lit, SolveResult, Solver};
use drat_check::{check_with_limits, Limits, Rejection};

/// The conflict bound of the route budget of VOS30 8.6.1.
pub const ROUTE_MAX_CONFLICTS: u64 = 50_000;

/// The `check_work` bound of the route budget of VOS30 8.6.1: half of the checker's default
/// work limit (2^32).
pub const ROUTE_MAX_CHECK_WORK: u64 = 1 << 31;

/// The two bounds an obligation must stay inside.  [`Budget::ROUTE`] is the only value the CLI
/// uses; the type exists so a test can hand the lane a deliberately wrong budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget {
    /// At most this many conflicts.
    pub max_conflicts: u64,
    /// At most this much checker work.
    pub max_check_work: u64,
}

impl Budget {
    /// The route budget of VOS30 8.6.1.
    pub const ROUTE: Budget = Budget {
        max_conflicts: ROUTE_MAX_CONFLICTS,
        max_check_work: ROUTE_MAX_CHECK_WORK,
    };
}

/// What the solver said.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Unsatisfiable.
    Unsat,
    /// Satisfiable, with the solver's model (one value per variable).
    Sat(Vec<bool>),
    /// The conflict bound ended the solve.
    Unknown,
}

/// The outcome of one solve.
#[derive(Debug)]
pub struct SolveOut {
    /// The verdict.
    pub verdict: Verdict,
    /// Conflicts used.
    pub conflicts: u64,
    /// The DRAT text of an UNSAT answer, when `proof` was asked for.
    pub drat: Option<String>,
}

/// Solve `cnf`, stopping after `max_conflicts` conflicts, with a DRAT proof iff `proof`.
///
/// The solver counts the budget down per call, so one call with `Some(max_conflicts)` is the
/// whole bound; an `Unknown` answer therefore means "over the budget", never "gave up early".
pub fn solve_cnf(cnf: &Cnf, max_conflicts: u64, proof: bool) -> SolveOut {
    let mut s = if proof {
        Solver::from_cnf_with_proof(cnf)
    } else {
        Solver::from_cnf(cnf)
    };
    s.set_conflict_budget(Some(max_conflicts));
    let verdict = match s.solve() {
        SolveResult::Sat => Verdict::Sat(s.model().to_vec()),
        SolveResult::Unsat => Verdict::Unsat,
        SolveResult::Unknown => Verdict::Unknown,
    };
    let drat = if proof && verdict == Verdict::Unsat {
        s.proof().map(|p| p.to_drat_text())
    } else {
        None
    };
    SolveOut {
        verdict,
        conflicts: s.stats().conflicts,
        drat,
    }
}

/// A check of a proof under the default limits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckOut {
    /// Accepted under `Limits::default()`.
    pub accepted: bool,
    /// `Accepted.work` when accepted: the `check_work` of the route budget (VOS31 section 16
    /// item 4: it is `drat-check`'s `work`, in literal visits, the unit `Limits::max_work` uses).
    pub work: Option<u64>,
    /// Why it was not accepted.
    pub reason: Option<String>,
}

/// Check `drat` against `cnf` under `Limits::default()`.  Never raises a limit.
pub fn check_default(cnf: &Cnf, drat: &str) -> CheckOut {
    let dimacs = write_dimacs(cnf);
    match check_with_limits(&dimacs, drat, Limits::default()) {
        Ok(a) => CheckOut {
            accepted: true,
            work: Some(a.work),
            reason: None,
        },
        Err(Rejection::ResourceLimit { step, .. }) => CheckOut {
            accepted: false,
            work: None,
            reason: Some(format!(
                "the default work limit was exceeded at proof step {step}"
            )),
        },
        Err(e) => CheckOut {
            accepted: false,
            work: None,
            reason: Some(format!("{e}")),
        },
    }
}

/// Whether `model` satisfies every clause of `cnf`, by a plain re-evaluation that trusts
/// neither the solver's flag nor `cdcl-sat`'s own evaluator.  A variable the model does not
/// cover counts as false.
pub fn model_satisfies(cnf: &Cnf, model: &[bool]) -> bool {
    cnf.clauses.iter().all(|clause| {
        clause.iter().any(|l| {
            let v = model.get(l.var().index()).copied().unwrap_or(false);
            v != l.is_negated()
        })
    })
}

/// The index of a literal in a table of `2 * num_vars` slots: `2 * var + negated`.
fn slot(l: Lit) -> usize {
    2 * l.var().index() + usize::from(l.is_negated())
}

/// Does **unit propagation alone** refute `cnf`?  (VOS31 7.5, the `propagation-only` reason.)
///
/// This is the lane's own routine, about forty lines, deliberately tiny and deterministic, so
/// that the committed `collapsed` field depends on no solver (section 8.1): a clause with no
/// literal left is a conflict; a clause with one unassigned literal and no true one forces it;
/// the loop runs to a fixed point.  No decision is ever made.
///
/// ```text
///   x          (unit)
///   -x v y     x is true, so -x is false: y is forced
///   -y         y is true, -y is false: the clause is empty: REFUTED
/// ```
pub fn propagation_refutes(cnf: &Cnf) -> bool {
    let nv = cnf.num_vars;
    // value[slot(l)]: 1 when `l` is true, -1 when false, 0 when unknown; both polarities of a
    // variable are kept in step.
    let mut value = vec![0i8; 2 * nv];
    // watching[slot(l)]: the clauses that contain NOT(l), the ones `l` becoming true can shorten.
    let mut watching: Vec<Vec<usize>> = vec![Vec::new(); 2 * nv];
    for (ci, clause) in cnf.clauses.iter().enumerate() {
        for &l in clause {
            watching[slot(!l)].push(ci);
        }
    }
    let mut queue: Vec<Lit> = Vec::new();
    // Look at one clause: `Err` is a conflict, a forced literal is queued.
    let examine = |ci: usize, value: &[i8], queue: &mut Vec<Lit>| -> Result<(), ()> {
        let mut free: Option<Lit> = None;
        let mut free_count = 0;
        for &l in &cnf.clauses[ci] {
            match value[slot(l)] {
                1 => return Ok(()),
                0 => {
                    free = Some(l);
                    free_count += 1;
                }
                _ => {}
            }
        }
        match (free_count, free) {
            (0, _) => Err(()),
            (1, Some(l)) => {
                queue.push(l);
                Ok(())
            }
            _ => Ok(()),
        }
    };
    for ci in 0..cnf.clauses.len() {
        if examine(ci, &value, &mut queue).is_err() {
            return true;
        }
    }
    while let Some(l) = queue.pop() {
        match value[slot(l)] {
            1 => continue,
            -1 => return true,
            _ => {}
        }
        value[slot(l)] = 1;
        value[slot(!l)] = -1;
        for &ci in &watching[slot(l)] {
            if examine(ci, &value, &mut queue).is_err() {
                return true;
            }
        }
    }
    false
}

/// `n` with thousands separators, as VOS30 writes it (`50,000`).
fn with_commas(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Does the text of VOS30 section 8.6.1 ("The route budget") state exactly `budget`?  VOS31 7.3:
/// the constants are frozen there, and a test reads the section so that a change to either
/// number fails this lane until the owner records the specification change.
///
/// The section must contain `**<conflicts with commas> conflicts**` and `**2^<k>**`, where the
/// work bound is the power of two `2^k`.  A work bound that is not a power of two is never stated
/// that way, so it never matches.
pub fn spec_states_budget(vos30: &str, budget: Budget) -> Result<(), String> {
    let start = vos30
        .find("#### 8.6.1 The route budget")
        .ok_or("VOS30 has no section 8.6.1 `The route budget`")?;
    let rest = &vos30[start + 4..];
    let section = &rest[..rest.find("\n#### ").unwrap_or(rest.len())];
    let conflicts = format!("**{} conflicts**", with_commas(budget.max_conflicts));
    if !section.contains(&conflicts) {
        return Err(format!("VOS30 8.6.1 does not state {conflicts}"));
    }
    let k = budget.max_check_work.trailing_zeros();
    if !budget.max_check_work.is_power_of_two() || !section.contains(&format!("**2^{k}**")) {
        return Err(format!(
            "VOS30 8.6.1 does not state a check_work bound of **2^{k}** ({})",
            budget.max_check_work
        ));
    }
    Ok(())
}
