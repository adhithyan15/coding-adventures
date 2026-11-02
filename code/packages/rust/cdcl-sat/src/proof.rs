//! # DRAT proofs: the solver's certificate of UNSAT
//!
//! **VER11 §4.** The solver is not trusted (VER00 §7.5). When it answers
//! UNSAT it can also hand over a **DRAT proof**: every clause it learned, in
//! the order it learned them, every learned clause it threw away, and
//! finally the empty clause. The `drat-check` crate (the trusted kernel)
//! replays the proof against the original formula and accepts only if every
//! learned clause really follows from what came before.
//!
//! ```text
//!   formula                 proof (DRAT text)
//!   p cnf 2 4               1 0          ← learned (x1)
//!   1 2 0                   d 1 2 0      ← deleted (x1 ∨ x2), no longer needed
//!   1 -2 0                  0            ← the empty clause: UNSAT
//!   -1 2 0
//!   -1 -2 0
//! ```
//!
//! The proof is collected **in memory**: the solver stays pure computation,
//! with no capability to write files. Writing [`DratProof::to_drat_text`] to
//! disk is the caller's job.
//!
//! Original clauses are never logged. They are the formula, and the checker
//! reads them from the formula itself.

use crate::lit::Lit;

/// One step of a DRAT proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofStep {
    /// A lemma: a clause implied by the formula and the lemmas before it
    /// (checkable by reverse unit propagation). Empty means UNSAT.
    Add(Vec<Lit>),
    /// A clause removed from the solver's database.
    Delete(Vec<Lit>),
}

/// A DRAT proof, in the order the solver produced it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DratProof {
    /// The steps, oldest first.
    pub steps: Vec<ProofStep>,
}

impl DratProof {
    /// Whether the proof ends the way a refutation must: by adding the
    /// empty clause.
    pub fn derives_empty_clause(&self) -> bool {
        self.steps.iter().any(|s| matches!(s, ProofStep::Add(c) if c.is_empty()))
    }

    /// The proof in DRAT text format, one step per line: literals in DIMACS
    /// numbering ended by `0`, deletions prefixed with `d`.
    pub fn to_drat_text(&self) -> String {
        let mut out = String::new();
        for step in &self.steps {
            let lits = match step {
                ProofStep::Add(lits) => lits,
                ProofStep::Delete(lits) => {
                    out.push_str("d ");
                    lits
                }
            };
            for l in lits {
                out.push_str(&l.to_string());
                out.push(' ');
            }
            out.push_str("0\n");
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(d: i64) -> Lit {
        Lit::from_dimacs(d).unwrap()
    }

    #[test]
    fn text_matches_the_module_example() {
        let p = DratProof {
            steps: vec![
                ProofStep::Add(vec![lit(1)]),
                ProofStep::Delete(vec![lit(1), lit(2)]),
                ProofStep::Add(vec![]),
            ],
        };
        assert_eq!(p.to_drat_text(), "1 0\nd 1 2 0\n0\n");
        assert!(p.derives_empty_clause());
        assert!(!DratProof::default().derives_empty_clause());
        assert_eq!(DratProof::default().to_drat_text(), "");
    }
}
