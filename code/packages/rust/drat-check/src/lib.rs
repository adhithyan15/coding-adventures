//! # `drat-check` — the trusted kernel for SAT proofs
//!
//! **VER11.** When the verified shipping program (VER00 §7.6) proves a
//! property, the proof is a SAT solver saying UNSAT. The solver is large and
//! clever, so it is **not trusted**. Instead it emits a DRAT certificate,
//! and this crate checks it. If this crate accepts, the formula is
//! unsatisfiable, whatever bugs the solver has.
//!
//! That makes this crate the one place where a bug could turn into a false
//! `proved` label, so it is built to be audited:
//!
//! - **No dependencies.** Not even on the solver: it reads DIMACS with its
//!   own code (`parse.rs`), so a parser bug cannot fool both sides alike.
//! - **Small.** The checking logic is `check.rs`, a few hundred lines with
//!   its explanations. Read it top to bottom.
//! - **Forward and stateless between steps.** Each lemma is checked by
//!   unit propagation from an empty assignment. No trail survives between
//!   checks, so there is no hidden state to get wrong.
//! - **Strict.** Truncated proofs, deletions of absent clauses, malformed
//!   input and proofs without an empty clause are all rejected.
//!
//! ## Usage
//!
//! ```
//! // (x1 ∨ x2)(x1 ∨ ¬x2)(¬x1 ∨ x2)(¬x1 ∨ ¬x2) is UNSAT.
//! let formula = "p cnf 2 4\n1 2 0\n1 -2 0\n-1 2 0\n-1 -2 0\n";
//!
//! // Learn (x1), then the empty clause.
//! let ok = drat_check::check(formula, "1 0\n0\n").unwrap();
//! assert_eq!(ok.lemmas, 2);
//!
//! // Skipping the lemma is not a proof: the empty clause is not RUP yet.
//! let err = drat_check::check(formula, "0\n").unwrap_err();
//! assert!(err.to_string().contains("neither RUP nor RAT"));
//! ```
//!
//! ## Where it sits
//!
//! ```text
//!   formula ──► cdcl-sat ──► UNSAT + DRAT proof ──┐
//!      │                                          ▼
//!      └────────────────────────────────────► drat-check ──► Accepted / Rejection
//! ```

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]
#![forbid(unsafe_code)]

mod check;
pub mod parse;

use std::fmt;

use check::Outcome;
pub use parse::ParseError;

/// The default cap on variable indices, in the formula and in the proof:
/// 2^20. The checker spends about 60 bytes per variable (two watch lists
/// and a value), so a hostile input can make it allocate at most about
/// 60 MB before checking a single step. Callers with bigger formulas raise
/// the limit with [`check_with_limit`].
///
/// Raising it is a memory trade-off: tables are sized by the largest
/// variable index either input mentions, so a limit of `n` lets a few bytes
/// of hostile proof cost about `60·n` bytes.
pub const DEFAULT_MAX_VARS: usize = 1 << 20;

/// The default work budget, in literal visits: 2^32. That is seconds to a
/// minute of checking, which covers every proof this repository's tests
/// produce many times over, while a hostile proof designed to be slow (RAT
/// steps with many partners, repeated over a large database) is cut off
/// instead of running for hours. Running out only ever rejects.
pub const DEFAULT_MAX_WORK: u64 = 1 << 32;

/// Bounds on what a check may cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Highest variable index accepted (capped at [`parse::MAX_VAR_LIMIT`]).
    pub max_vars: usize,
    /// Most work, in literal visits, before the check gives up and rejects.
    pub max_work: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Limits { max_vars: DEFAULT_MAX_VARS, max_work: DEFAULT_MAX_WORK }
    }
}

/// A proof that passed: the formula is unsatisfiable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Accepted {
    /// Lemmas checked and added, the empty clause included.
    pub lemmas: usize,
    /// How many of them needed the RAT check (were not RUP).
    pub rat_lemmas: usize,
    /// Deletions performed.
    pub deletions: usize,
    /// Proof line of the empty clause.
    pub empty_clause_line: usize,
    /// Work spent, in literal visits (compare [`Limits::max_work`]).
    pub work: u64,
}

/// Why a proof was not accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rejection {
    /// The formula or the proof could not be read.
    Parse(ParseError),
    /// A lemma is neither RUP nor RAT.
    NotImplied {
        /// 0-based step index.
        step: usize,
        /// Proof line.
        line: usize,
    },
    /// A deletion names a clause that is not in the database.
    DeleteAbsent {
        /// 0-based step index.
        step: usize,
        /// Proof line.
        line: usize,
    },
    /// The proof never adds the empty clause.
    NoEmptyClause,
    /// The work budget ran out while checking a step. The proof may be
    /// valid; it was not shown to be within the budget.
    ResourceLimit {
        /// 0-based step index.
        step: usize,
        /// Proof line.
        line: usize,
    },
}

impl fmt::Display for Rejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Rejection::Parse(e) => write!(f, "{e}"),
            Rejection::NotImplied { step, line } => {
                write!(f, "proof line {line}: lemma (step {step}) is neither RUP nor RAT")
            }
            Rejection::DeleteAbsent { step, line } => {
                write!(f, "proof line {line}: deletion (step {step}) of a clause that is not present")
            }
            Rejection::NoEmptyClause => write!(f, "the proof never derives the empty clause"),
            Rejection::ResourceLimit { step, line } => {
                write!(f, "proof line {line}: work budget exhausted while checking step {step}")
            }
        }
    }
}

impl std::error::Error for Rejection {}

/// Check a DRAT `proof` against a DIMACS `formula`, with the default
/// [`Limits`].
pub fn check(formula: &str, proof: &str) -> Result<Accepted, Rejection> {
    check_with_limits(formula, proof, Limits::default())
}

/// Check with a custom variable limit and the default work budget.
pub fn check_with_limit(formula: &str, proof: &str, max_vars: usize) -> Result<Accepted, Rejection> {
    check_with_limits(formula, proof, Limits { max_vars, ..Limits::default() })
}

/// Check a DRAT `proof` against a DIMACS `formula` within `limits`:
/// variable indices above `limits.max_vars` (itself capped at
/// [`parse::MAX_VAR_LIMIT`]) are rejected, and so is a proof whose check
/// needs more than `limits.max_work`.
pub fn check_with_limits(formula: &str, proof: &str, limits: Limits) -> Result<Accepted, Rejection> {
    let max_vars = limits.max_vars.min(parse::MAX_VAR_LIMIT);
    let f = parse::parse_formula(formula, max_vars).map_err(Rejection::Parse)?;
    let steps = parse::parse_proof(proof, max_vars).map_err(Rejection::Parse)?;
    // Size the tables for every variable either input mentions.
    let proof_vars = steps
        .iter()
        .flat_map(|s| s.lits.iter())
        .map(|&l| parse::var(l) + 1)
        .max()
        .unwrap_or(0);
    let num_vars = f.num_vars.max(proof_vars);
    match check::run(num_vars, &f.clauses, &steps, limits.max_work) {
        Outcome::Refuted { step, lemmas, rat_lemmas, deletions, work } => Ok(Accepted {
            lemmas,
            rat_lemmas,
            deletions,
            empty_clause_line: steps[step].line,
            work,
        }),
        Outcome::ResourceLimit { step } => Err(Rejection::ResourceLimit { step, line: steps[step].line }),
        Outcome::NotImplied { step } => Err(Rejection::NotImplied { step, line: steps[step].line }),
        Outcome::DeleteAbsent { step } => Err(Rejection::DeleteAbsent { step, line: steps[step].line }),
        Outcome::NoEmptyClause => Err(Rejection::NoEmptyClause),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const XOR_UNSAT: &str = "p cnf 2 4\n1 2 0\n1 -2 0\n-1 2 0\n-1 -2 0\n";

    #[test]
    fn accepts_and_reports() {
        let ok = check(XOR_UNSAT, "c proof\n1 0\nd 1 2 0\n0\n").unwrap();
        assert_eq!(ok, Accepted { lemmas: 2, rat_lemmas: 0, deletions: 1, empty_clause_line: 4, work: ok.work });
        assert!(ok.work > 0);
        let tight = Limits { max_work: ok.work - 1, ..Limits::default() };
        assert_eq!(
            check_with_limits(XOR_UNSAT, "c proof\n1 0\nd 1 2 0\n0\n", tight),
            Err(Rejection::ResourceLimit { step: 2, line: 4 })
        );
    }

    #[test]
    fn every_rejection_kind() {
        assert!(matches!(check("p cnf 1 1\n", "0\n"), Err(Rejection::Parse(ParseError::Formula { .. }))));
        assert!(check(XOR_UNSAT, "1 0\n0").is_ok());
        assert_eq!(
            check(XOR_UNSAT, "1 0\n1"),
            Err(Rejection::Parse(ParseError::Truncated { line: 2 }))
        );
        assert_eq!(check(XOR_UNSAT, "0\n"), Err(Rejection::NotImplied { step: 0, line: 1 }));
        assert_eq!(check(XOR_UNSAT, "1 0\nd 1 0\nd 1 0\n"), Err(Rejection::DeleteAbsent { step: 2, line: 3 }));
        assert_eq!(check(XOR_UNSAT, "1 0\n"), Err(Rejection::NoEmptyClause));
        assert_eq!(check(XOR_UNSAT, ""), Err(Rejection::NoEmptyClause));
    }

    #[test]
    fn proof_variables_beyond_the_formula_and_the_limit() {
        // x5 does not occur in the formula; a RAT lemma may introduce it.
        let ok = check(XOR_UNSAT, "5 0\n1 0\n0\n").unwrap();
        assert_eq!(ok.rat_lemmas, 1);
        assert!(matches!(
            check_with_limit(XOR_UNSAT, "5 0\n", 4),
            Err(Rejection::Parse(ParseError::ProofVarOverLimit { .. }))
        ));
        // The limit itself is capped so codes fit in u32.
        assert!(check_with_limit(XOR_UNSAT, "1 0\n0\n", usize::MAX).is_ok());
    }

    #[test]
    fn rejections_display() {
        let msgs = [
            Rejection::NotImplied { step: 3, line: 7 }.to_string(),
            Rejection::DeleteAbsent { step: 1, line: 2 }.to_string(),
            Rejection::NoEmptyClause.to_string(),
            Rejection::Parse(ParseError::Truncated { line: 9 }).to_string(),
            Rejection::ResourceLimit { step: 4, line: 5 }.to_string(),
        ];
        assert!(msgs[4].contains("budget"));
        assert!(msgs[0].contains("line 7") && msgs[0].contains("step 3"));
        assert!(msgs[1].contains("not present"));
        assert!(msgs[2].contains("empty clause"));
        assert!(msgs[3].contains("truncated"));
    }
}
