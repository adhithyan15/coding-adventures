//! # `solve`: the budgeted solve loop, the proof, and the checker under its default limits
//!
//! Three rules of VOS30 section 9.5 live here:
//!
//! 1. **Conflict cap with proof logging.**  The proof is held in memory (about 350 MB per
//!    million conflicts), so with logging on the solve stops at [`PROOF_CONFLICT_CAP`]; past
//!    it the answer is `Unknown` with cap `conflicts`.
//! 2. **Wall timeout as a loop of slices.**  `set_conflict_budget` counts down per call, so
//!    the loop resets it to [`CONFLICT_SLICE`] each iteration and looks at the clock between
//!    slices.  A timeout is `Unknown` with cap `wall` and the conflicts used so far.
//! 3. **The checker runs under `Limits::default()` and the probe never raises them.**  A
//!    proof the default limits reject is recorded as `pass_default_limits = false` with the
//!    reason; it is a red row, never a capacity datum for a bigger limit (VOS30 section 6.3).

use cdcl_sat::{write_dimacs, Cnf, SolveResult, Solver};
use drat_check::{check_with_limits, Limits, Rejection};
use std::time::{Duration, Instant};

/// Conflicts per slice of the solve loop (VOS30 section 9.5).
pub const CONFLICT_SLICE: u64 = 20_000;
/// Watch-list visits per slice: a slice also ends here, so that the clock is read about every
/// second even on a million-clause formula where 20,000 conflicts would take minutes.  The
/// budget is per `solve` call and the loop starts a new slice after each `Unknown`, so it only
/// bounds the time between clock reads; it cannot change an answer (the solver resumes).
pub const WORK_SLICE: u64 = 400_000_000;
/// The conflict cap whenever proof logging is on.
pub const PROOF_CONFLICT_CAP: u64 = 1_000_000;

/// Which limit ended a solve.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cap {
    /// Nothing: the solver answered.
    None,
    /// The conflict cap.
    Conflicts,
    /// The wall clock.
    Wall,
}

impl Cap {
    /// The name recorded in the results file.
    pub fn name(self) -> &'static str {
        match self {
            Cap::None => "none",
            Cap::Conflicts => "conflicts",
            Cap::Wall => "wall",
        }
    }
}

/// What the solver said.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Unsatisfiable.
    Unsat,
    /// Satisfiable, with the solver's model.
    Sat(Vec<bool>),
    /// A budget ended the solve.
    Unknown(Cap),
}

/// Options of one solve.
#[derive(Clone, Copy, Debug)]
pub struct SolveOpts {
    /// Stop at this instant.
    pub deadline: Instant,
    /// Log a DRAT proof.
    pub proof: bool,
    /// An additional conflict cap (the SAT controls use it).
    pub conflict_cap: Option<u64>,
}

impl SolveOpts {
    /// Options with a proof and `wall` seconds from now.
    pub fn certified(wall: Duration) -> SolveOpts {
        SolveOpts {
            deadline: Instant::now() + wall,
            proof: true,
            conflict_cap: None,
        }
    }
}

/// The outcome of one solve.
#[derive(Debug)]
pub struct SolveOut {
    /// The verdict.
    pub verdict: Verdict,
    /// Conflicts used.
    pub conflicts: u64,
    /// Propagations.
    pub propagations: u64,
    /// Wall time of the solve loop in milliseconds.
    pub solve_ms: f64,
    /// The DRAT text, for an UNSAT answer with logging on.
    pub drat: Option<String>,
}

/// Solve `cnf` under the limits of section 9.5.
pub fn solve_cnf(cnf: &Cnf, o: &SolveOpts) -> SolveOut {
    let t0 = Instant::now();
    let mut s = if o.proof {
        Solver::from_cnf_with_proof(cnf)
    } else {
        Solver::from_cnf(cnf)
    };
    let (verdict, cap) = loop {
        s.set_conflict_budget(Some(CONFLICT_SLICE));
        s.set_work_budget(Some(WORK_SLICE));
        match s.solve() {
            SolveResult::Sat => break (Verdict::Sat(s.model().to_vec()), Cap::None),
            SolveResult::Unsat => break (Verdict::Unsat, Cap::None),
            SolveResult::Unknown => {
                let c = s.stats().conflicts;
                if o.proof && c >= PROOF_CONFLICT_CAP {
                    break (Verdict::Unknown(Cap::Conflicts), Cap::Conflicts);
                }
                if o.conflict_cap.is_some_and(|cap| c >= cap) {
                    break (Verdict::Unknown(Cap::Conflicts), Cap::Conflicts);
                }
                if Instant::now() >= o.deadline {
                    break (Verdict::Unknown(Cap::Wall), Cap::Wall);
                }
            }
        }
    };
    let _ = cap;
    let st = s.stats();
    let drat = if o.proof && verdict == Verdict::Unsat {
        s.proof().map(|p| p.to_drat_text())
    } else {
        None
    };
    SolveOut {
        verdict,
        conflicts: st.conflicts,
        propagations: st.propagations,
        solve_ms: t0.elapsed().as_secs_f64() * 1000.0,
        drat,
    }
}

/// Solve an incrementally-loaded `solver` under `assumptions` with the same slice loop and
/// wall clock as [`solve_cnf`] (no proof: the incremental mode is informational, section 9.5).
/// Returns the verdict and the conflicts this call used.
pub fn solve_assuming(
    s: &mut Solver,
    assumptions: &[cdcl_sat::Lit],
    deadline: Instant,
) -> (Verdict, u64) {
    let before = s.stats().conflicts;
    let verdict = loop {
        s.set_conflict_budget(Some(CONFLICT_SLICE));
        s.set_work_budget(Some(WORK_SLICE));
        match s.solve_with_assumptions(assumptions) {
            SolveResult::Sat => break Verdict::Sat(s.model().to_vec()),
            SolveResult::Unsat => break Verdict::Unsat,
            SolveResult::Unknown => {
                if Instant::now() >= deadline {
                    break Verdict::Unknown(Cap::Wall);
                }
            }
        }
    };
    (verdict, s.stats().conflicts - before)
}

/// A check of a proof under the default limits.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckOut {
    /// Accepted under `Limits::default()`.
    pub accepted: bool,
    /// `Accepted.work` when accepted.
    pub work: Option<u64>,
    /// Wall time of the check in milliseconds.
    pub ms: f64,
    /// Why it was not accepted.
    pub reason: Option<String>,
    /// True if the checker found the proof *wrong* (not merely over a limit): a solver bug.
    pub invalid: bool,
}

/// Check `drat` against `cnf` with `Limits::default()`: the probe never raises a limit.
pub fn check_default(cnf: &Cnf, drat: &str) -> CheckOut {
    let t0 = Instant::now();
    let dimacs = write_dimacs(cnf);
    let r = check_with_limits(&dimacs, drat, Limits::default());
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    match r {
        Ok(a) => CheckOut {
            accepted: true,
            work: Some(a.work),
            ms,
            reason: None,
            invalid: false,
        },
        Err(Rejection::ResourceLimit { step, .. }) => CheckOut {
            accepted: false,
            work: None,
            ms,
            reason: Some(format!("default work limit 2^32 exceeded at step {step}")),
            invalid: false,
        },
        Err(Rejection::Parse(e)) => CheckOut {
            accepted: false,
            work: None,
            ms,
            reason: Some(format!("default limits reject the input: {e}")),
            invalid: false,
        },
        Err(e) => CheckOut {
            accepted: false,
            work: None,
            ms,
            reason: Some(format!("checker: {e}")),
            invalid: true,
        },
    }
}

/// Whether `model` satisfies every clause of `cnf`, by a plain re-evaluation that trusts
/// neither the solver's flag nor `cdcl-sat`'s own evaluator (guard G3).
pub fn model_satisfies(cnf: &Cnf, model: &[bool]) -> bool {
    'clauses: for clause in &cnf.clauses {
        for l in clause {
            let v = model.get(l.var().index()).copied().unwrap_or(false);
            if v != l.is_negated() {
                continue 'clauses;
            }
        }
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use cdcl_sat::Var;

    fn pigeonhole(n: usize) -> Cnf {
        // n+1 pigeons, n holes: UNSAT and needs real search.
        let var = |p: usize, h: usize| Var::from_index(p * n + h);
        let mut c = Cnf::new(0);
        for p in 0..=n {
            let lits: Vec<_> = (0..n).map(|h| var(p, h).positive()).collect();
            c.add_clause(&lits);
        }
        for h in 0..n {
            for p in 0..=n {
                for q in p + 1..=n {
                    c.add_clause(&[var(p, h).negative(), var(q, h).negative()]);
                }
            }
        }
        c
    }

    #[test]
    fn an_unsat_instance_yields_a_proof_the_default_checker_accepts() {
        let cnf = pigeonhole(5);
        let out = solve_cnf(&cnf, &SolveOpts::certified(Duration::from_secs(20)));
        assert_eq!(out.verdict, Verdict::Unsat);
        assert!(out.conflicts > 0);
        let chk = check_default(&cnf, out.drat.as_ref().unwrap());
        assert!(chk.accepted, "{chk:?}");
        assert!(chk.work.unwrap() > 0);
    }

    #[test]
    fn a_sat_instance_yields_a_model_that_a_plain_evaluator_accepts() {
        let mut c = Cnf::new(0);
        let (x, y) = (Var::from_index(0), Var::from_index(1));
        c.add_clause(&[x.positive(), y.positive()]);
        c.add_clause(&[x.negative()]);
        let out = solve_cnf(
            &c,
            &SolveOpts {
                deadline: Instant::now() + Duration::from_secs(5),
                proof: false,
                conflict_cap: None,
            },
        );
        let Verdict::Sat(m) = out.verdict else {
            panic!("expected sat")
        };
        assert!(model_satisfies(&c, &m));
        let mut bad = m.clone();
        bad[1] = false;
        assert!(!model_satisfies(&c, &bad));
    }

    #[test]
    fn the_wall_clock_ends_a_solve_with_the_conflicts_so_far() {
        // A deadline already past: the first slice runs, then the loop must stop (cap wall).
        let cnf = pigeonhole(9);
        let o = SolveOpts {
            deadline: Instant::now(),
            proof: false,
            conflict_cap: None,
        };
        let out = solve_cnf(&cnf, &o);
        assert_eq!(out.verdict, Verdict::Unknown(Cap::Wall));
        assert!(out.conflicts >= CONFLICT_SLICE);
    }

    #[test]
    fn an_explicit_conflict_cap_ends_a_solve_with_cap_conflicts() {
        let cnf = pigeonhole(9);
        let o = SolveOpts {
            deadline: Instant::now() + Duration::from_secs(60),
            proof: false,
            conflict_cap: Some(1000),
        };
        let out = solve_cnf(&cnf, &o);
        assert_eq!(out.verdict, Verdict::Unknown(Cap::Conflicts));
    }

    #[test]
    fn assumptions_make_one_solver_answer_two_segments_in_turn() {
        let mut s = Solver::new();
        let (x, y) = (Var::from_index(0), Var::from_index(1));
        s.add_clause(&[x.positive(), y.positive()]);
        let deadline = Instant::now() + Duration::from_secs(5);
        // Assuming both false contradicts the clause; assuming x true is fine.
        let (v, _) = solve_assuming(&mut s, &[x.negative(), y.negative()], deadline);
        assert_eq!(v, Verdict::Unsat);
        let (v, _) = solve_assuming(&mut s, &[x.positive()], deadline);
        assert!(matches!(v, Verdict::Sat(_)));
        let past = Instant::now();
        let mut hard = Solver::from_cnf(&pigeonhole(9));
        let (v, c) = solve_assuming(&mut hard, &[], past);
        assert_eq!(v, Verdict::Unknown(Cap::Wall));
        assert!(c >= CONFLICT_SLICE);
    }

    #[test]
    fn a_proof_that_does_not_refute_is_reported_invalid_not_as_a_limit() {
        let cnf = pigeonhole(3);
        let chk = check_default(&cnf, "1 0\n");
        assert!(!chk.accepted);
        assert!(chk.invalid);
        assert_eq!(Cap::Wall.name(), "wall");
        assert_eq!(Cap::Conflicts.name(), "conflicts");
        assert_eq!(Cap::None.name(), "none");
    }

    #[test]
    fn a_formula_over_the_default_variable_limit_is_rejected_by_the_default_limits() {
        // 2^20 + 2 variables: the default kernel refuses the formula itself.
        let mut c = Cnf::new(0);
        c.add_clause(&[Var::from_index((1 << 20) + 1).positive()]);
        let chk = check_default(&c, "0\n");
        assert!(!chk.accepted);
        assert!(!chk.invalid, "a limit is not a wrong proof: {chk:?}");
        assert!(chk.reason.unwrap().contains("default limits"));
    }
}
