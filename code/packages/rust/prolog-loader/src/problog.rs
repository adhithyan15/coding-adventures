//! # `problog` — builder API for probabilistic Prolog programs
//!
//! Bridges the gap between the **engine half** of the ProbLog story
//! (which already works — `logic_engine` ships
//! `Fact::with_probability` / `Rule::with_probability` plus the WMC
//! backend) and the **source half** (which still needs grammar work
//! to accept `0.7::edge(a, b).`-style syntax).
//!
//! Until the grammar lands, callers can compose a probabilistic
//! program declaratively via [`ProblogProgram`] and exercise the
//! end-to-end engine path without touching the parser.
//!
//! ## What "probabilistic Prolog" means here
//!
//! Each Fact carries an annotation `Certain` or `Value(p)`. Each
//! Rule does too. The engine runs `search(query, kb, AutoDetect)`:
//!
//! - If every clause used in any proof is `Certain`, the cheap
//!   `FindFirst` path returns at most one binding.
//! - If any clause is probabilistic, the `EnumerateAll` path returns
//!   a `ProofDAG` + the engine's WMC.
//!
//! That gives `P(query)`, the standard ProbLog semantics, under the
//! independence assumption baked into the WMC implementation
//! (`logic_engine::wmc`).
//!
//! ## When the grammar supports `0.7::` source syntax
//!
//! This builder stays useful regardless — it's the **typed API**
//! consumers and tests reach for to construct programs without
//! round-tripping through text. A future `load_problog_source(src)`
//! will be a thin wrapper that converts parsed clauses into
//! `ProblogProgram::with_prob_fact` / `with_prob_rule` calls.

use logic_core::Term;
use logic_engine::{search, BodyLiteral, Fact, KnowledgeBase, Rule, SearchMode};

use crate::QueryRun;

/// A probabilistic Prolog program under construction. Mirrors a
/// `.pl` file: facts, rules, and queries. Builder methods are
/// chainable. Drive to completion with [`Self::execute`].
#[derive(Debug, Default)]
pub struct ProblogProgram {
    facts: Vec<Fact>,
    rules: Vec<Rule>,
    queries: Vec<Vec<Term>>,
}

impl ProblogProgram {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a deterministic fact. Same semantics as
    /// `Fact::certain(term)`.
    pub fn with_fact(mut self, term: Term) -> Self {
        self.facts.push(Fact::certain(term));
        self
    }

    /// Add a probabilistic fact: `p :: term.`. `p` must be in
    /// `[0, 1]`. Out-of-range values are accepted as-is so callers
    /// can deliberately encode boundary conditions in tests; the WMC
    /// backend treats anything outside `[0, 1]` as a programmer
    /// error.
    pub fn with_prob_fact(mut self, p: f64, term: Term) -> Self {
        self.facts.push(Fact::with_probability(term, p));
        self
    }

    /// Add a deterministic rule `head :- body[0], body[1], ...`.
    pub fn with_rule(mut self, head: Term, body: Vec<BodyLiteral>) -> Self {
        self.rules.push(Rule::certain(head, body));
        self
    }

    /// Add a probabilistic rule `p :: head :- body[0], ...`. The
    /// rule's whole derivation is gated on a Bernoulli coin: with
    /// probability `p` the rule fires, with probability `1 - p` it
    /// does not.
    pub fn with_prob_rule(mut self, p: f64, head: Term, body: Vec<BodyLiteral>) -> Self {
        self.rules.push(Rule::with_probability(head, body, p));
        self
    }

    /// Add a top-level `?- g1, g2, ..., gn.` query.
    pub fn with_query(mut self, goals: Vec<Term>) -> Self {
        self.queries.push(goals);
        self
    }

    /// Build a [`KnowledgeBase`] without running any queries. Useful
    /// for callers that want to add more clauses programmatically
    /// after the builder hands them the KB.
    pub fn build_kb(self) -> (KnowledgeBase, Vec<Vec<Term>>) {
        let mut kb = KnowledgeBase::new();
        for f in self.facts {
            kb.add_fact(f);
        }
        for r in self.rules {
            kb.add_rule(r);
        }
        (kb, self.queries)
    }

    /// Build the KB, run every query through `logic_engine::search`,
    /// and return the resulting [`QueryRun`]s alongside the KB. The
    /// default search mode is `AutoDetect`, which uses the cheap
    /// `FindFirst` path on a certain-only KB and falls back to
    /// `EnumerateAll` + WMC when any clause is probabilistic.
    pub fn execute(self) -> (KnowledgeBase, Vec<QueryRun>) {
        self.execute_with_mode(SearchMode::AutoDetect)
    }

    /// Same as [`Self::execute`] but with an explicit search mode.
    /// Tests use this to force `EnumerateAll` on a certain-only
    /// program so they can read back the WMC value (which is `1.0`
    /// on a deterministic proof).
    pub fn execute_with_mode(self, mode: SearchMode) -> (KnowledgeBase, Vec<QueryRun>) {
        let (mut kb, queries) = self.build_kb();
        let mut runs = Vec::with_capacity(queries.len());
        for (i, goals) in queries.into_iter().enumerate() {
            let searched = match goals.len() {
                0 => Term::Atom("true".to_string()),
                1 => goals[0].clone(),
                _ => {
                    // Same synthetic-head rewrite as the deterministic
                    // `run_all_queries` path. `__problog_query_N` cannot
                    // collide with user source because identifiers
                    // starting with `_` are variables, not atoms.
                    let head = Term::Atom(format!("__problog_query_{i}"));
                    let body: Vec<BodyLiteral> =
                        goals.iter().cloned().map(BodyLiteral::Pos).collect();
                    kb.add_rule(Rule::certain(head.clone(), body));
                    head
                }
            };
            let result = search(&searched, &kb, mode);
            runs.push(QueryRun {
                goals,
                searched,
                result,
            });
        }
        (kb, runs)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use logic_core::{atom, compound, var};
    use logic_engine::{BodyLiteral, SearchResult};

    fn assert_close(actual: f64, expected: f64, tol: f64) {
        assert!(
            (actual - expected).abs() < tol,
            "expected {expected} ± {tol}, got {actual}",
        );
    }

    #[test]
    fn deterministic_fact_via_builder_succeeds_with_probability_one() {
        // The builder still works for fully deterministic programs;
        // ProbLog should subsume Prolog.
        let (_kb, runs) = ProblogProgram::new()
            .with_fact(atom("sunny"))
            .with_query(vec![atom("sunny")])
            .execute();
        assert_eq!(runs.len(), 1);
        assert!(runs[0].succeeded());
        assert_eq!(runs[0].probability(), 1.0);
    }

    #[test]
    fn single_probabilistic_fact_returns_its_probability() {
        // The textbook ProbLog "Bernoulli coin" example: a single
        // probabilistic fact, query it, get its probability back.
        let (_kb, runs) = ProblogProgram::new()
            .with_prob_fact(0.7, atom("rain"))
            .with_query(vec![atom("rain")])
            .execute();
        assert_eq!(runs.len(), 1);
        assert!(runs[0].succeeded(), "rain has positive probability");
        assert_close(runs[0].probability(), 0.7, 1e-9);
    }

    #[test]
    fn two_independent_probabilistic_facts_in_a_conjunctive_rule_multiply() {
        // Independence: P(alarm) = P(burglary) * P(triggers_alarm)
        // when the only rule is `alarm :- burglary, triggers_alarm.`
        // Both probabilistic facts are independent in the WMC's
        // distribution semantics.
        let (_kb, runs) = ProblogProgram::new()
            .with_prob_fact(0.1, atom("burglary"))
            .with_prob_fact(0.9, atom("triggers_alarm"))
            .with_rule(
                atom("alarm"),
                vec![
                    BodyLiteral::Pos(atom("burglary")),
                    BodyLiteral::Pos(atom("triggers_alarm")),
                ],
            )
            .with_query(vec![atom("alarm")])
            .execute();
        assert_eq!(runs.len(), 1);
        assert!(runs[0].succeeded());
        assert_close(runs[0].probability(), 0.1 * 0.9, 1e-9);
    }

    #[test]
    fn probabilistic_rule_attenuates_a_certain_premise() {
        // `0.4 :: smoker(X) :- adult(X).` plus `adult(alice).`
        // gives P(smoker(alice)) = 0.4 — the rule's coin gates the
        // derivation, the premise is fully certain.
        let (_kb, runs) = ProblogProgram::new()
            .with_fact(compound("adult", vec![atom("alice")]))
            .with_prob_rule(
                0.4,
                compound("smoker", vec![Term::Var(var("X"))]),
                vec![BodyLiteral::Pos(compound("adult", vec![Term::Var(var("X"))]))],
            )
            .with_query(vec![compound("smoker", vec![atom("alice")])])
            .execute();
        assert_eq!(runs.len(), 1);
        assert!(runs[0].succeeded());
        assert_close(runs[0].probability(), 0.4, 1e-9);
    }

    #[test]
    fn unprovable_query_returns_zero() {
        // No proof at all → P = 0.
        let (_kb, runs) = ProblogProgram::new()
            .with_prob_fact(0.5, atom("rain"))
            .with_query(vec![atom("snow")])
            .execute();
        assert_eq!(runs.len(), 1);
        assert!(!runs[0].succeeded());
        assert_eq!(runs[0].probability(), 0.0);
    }

    #[test]
    fn boundary_probability_one_behaves_like_certain() {
        // A fact at p=1.0 should still be provable; the WMC value
        // is exactly 1.0 (no floating-point drift on this boundary).
        let (_kb, runs) = ProblogProgram::new()
            .with_prob_fact(1.0, atom("guaranteed"))
            .with_query(vec![atom("guaranteed")])
            .execute_with_mode(SearchMode::EnumerateAll);
        assert!(runs[0].succeeded());
        assert_close(runs[0].probability(), 1.0, 1e-12);
    }

    #[test]
    fn boundary_probability_zero_makes_query_fail() {
        // A fact at p=0.0 is degenerate — provable in proof space
        // but with zero probability mass.
        let (_kb, runs) = ProblogProgram::new()
            .with_prob_fact(0.0, atom("impossible"))
            .with_query(vec![atom("impossible")])
            .execute_with_mode(SearchMode::EnumerateAll);
        assert!(!runs[0].succeeded(), "P=0 ⇒ no probability mass on this query");
        assert_close(runs[0].probability(), 0.0, 1e-12);
    }

    #[test]
    fn build_kb_returns_kb_and_queries_without_executing() {
        // The non-running build path lets callers extend the KB
        // before search.
        let (mut kb, queries) = ProblogProgram::new()
            .with_prob_fact(0.3, atom("a"))
            .with_query(vec![atom("a")])
            .build_kb();
        assert_eq!(queries.len(), 1);
        // Add a second probabilistic fact after the fact (sic).
        kb.add_fact(Fact::with_probability(atom("b"), 0.5));
        // Sanity-check that the engine still works.
        let r = search(&atom("b"), &kb, SearchMode::EnumerateAll);
        match r {
            SearchResult::EnumerateAllResult { probability, .. } => {
                assert_close(probability, 0.5, 1e-9);
            }
            other => panic!("expected EnumerateAllResult, got {other:?}"),
        }
    }

    #[test]
    fn multi_goal_query_uses_synthetic_head_and_multiplies_probabilities() {
        // Two independent probabilistic facts in a query. The
        // synthetic head wraps the conjunction in a rule so the
        // engine can run it.
        let (_kb, runs) = ProblogProgram::new()
            .with_prob_fact(0.4, atom("p"))
            .with_prob_fact(0.25, atom("q"))
            .with_query(vec![atom("p"), atom("q")])
            .execute();
        assert_eq!(runs.len(), 1);
        assert_close(runs[0].probability(), 0.4 * 0.25, 1e-9);
    }
}
