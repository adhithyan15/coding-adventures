//! # `cdcl-sat` — a conflict-driven clause-learning SAT solver
//!
//! **VER09.** The Boolean engine under the verified shipping program
//! (VER00 §7.5 layer 1, §7.6). Every first-generation proof in that program
//! is a question to this crate: *can this formula be made true?* "No" (UNSAT)
//! is a proof over all inputs, once VER11's checker has verified the
//! certificate; "yes" (SAT) comes with a counterexample.
//!
//! ## Where it sits
//!
//! ```text
//!   VER10 encodings (Tseitin, AIG, bit-blasting)      constraint-engine
//!                │                                   (SatTactic adapter)
//!                ▼                                          │
//!          cdcl-sat  (this crate) ◄─────────────────────────┘
//!                │
//!                ▼  (VER11) DRAT proof  →  drat-check (the trusted kernel)
//! ```
//!
//! The solver is **not** trusted. The checker is. That is why the solver
//! may use every trick that makes it fast, and why it needs no proof of its
//! own correctness: a wrong UNSAT answer would come with a certificate the
//! checker rejects.
//!
//! ## From DPLL to CDCL, in one page
//!
//! The classic **DPLL** procedure (1962) guesses a value for a variable,
//! simplifies, and backtracks one guess at a time when it hits a
//! contradiction. It learns nothing from a contradiction, so it can hit the
//! same one again and again in different corners of the search.
//!
//! **CDCL** (GRASP, 1996; Chaff, 2001; MiniSat, 2003) changes three things:
//!
//! 1. **Learning.** When a clause becomes all-false, the solver works out
//!    *why*: which earlier decisions together forced it. It adds that
//!    explanation as a new clause, so the same combination is never tried
//!    again anywhere in the search.
//! 2. **Backjumping.** It then undoes every decision that had nothing to do
//!    with the conflict in one go, instead of one level at a time.
//! 3. **Laziness.** Two watched literals make propagation cost proportional
//!    to the clauses that might actually change, VSIDS focuses decisions on
//!    the variables in recent conflicts, and restarts let the solver reconsider
//!    its early choices without losing what it learned.
//!
//! Those three ideas take SAT solving from hundreds of variables to millions.
//!
//! ## Using it
//!
//! ```
//! use cdcl_sat::{parse_dimacs, Solver, SolveResult};
//!
//! // (x1 ∨ x2) ∧ (¬x1 ∨ x2) ∧ (¬x2 ∨ x3)
//! let cnf = parse_dimacs("p cnf 3 3\n1 2 0\n-1 2 0\n-2 3 0\n").unwrap();
//! let mut solver = Solver::from_cnf(&cnf);
//! assert_eq!(solver.solve(), SolveResult::Sat);
//! assert!(cnf.is_satisfied_by(solver.model()));
//! ```
//!
//! ## Modules
//!
//! | Module | Contents |
//! |---|---|
//! | [`lit`] | `Var`, `Lit` (packed `2·var + sign`), three-valued `LBool` |
//! | [`solver`] | the CDCL search: propagation, analysis, heuristics, assumptions |
//! | [`dimacs`] | `Cnf`, the strict DIMACS reader, the writer |
//! | `heap` | the VSIDS activity heap (internal) |
//! | [`luby`] | the Luby restart sequence |

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]
#![forbid(unsafe_code)]

pub mod dimacs;
mod heap;
pub mod lit;
pub mod luby;
pub mod solver;

pub use dimacs::{parse_dimacs, parse_dimacs_with_limit, write_dimacs, Cnf, DimacsError, DimacsErrorKind};
pub use lit::{LBool, Lit, Var};
pub use solver::{SolveResult, Solver, Stats};

impl Solver {
    /// A solver loaded with every clause of `cnf`, and with all of its
    /// variables created (even ones no clause mentions).
    pub fn from_cnf(cnf: &Cnf) -> Solver {
        let mut s = Solver::new();
        s.reserve_vars(cnf.num_vars);
        for clause in &cnf.clauses {
            s.add_clause(clause);
        }
        s
    }
}
