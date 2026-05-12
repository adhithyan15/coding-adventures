//! # logic-engine — probability-aware facts, rules, and find-first search.
//!
//! This is the first slice of [`LP19`](../../../specs/LP19-probabilistic-logic-core.md).
//! It builds on `logic-core` (LP00) and adds the next layer: clauses
//! (Facts and Rules), a knowledge base, and a deterministic search engine
//! that returns the first successful substitution.
//!
//! ## The Probability-First Design
//!
//! Every clause in this crate carries a [`Probability`] field. The default
//! is [`Probability::Certain`] — semantic 1.0 — so deterministic Prolog
//! is what you get when you never use any other value. A future slice of
//! this crate will expose [`KnowledgeBase::is_all_certain`] as the gate
//! that selects between the deterministic find-first path (this slice)
//! and the probabilistic proof-enumeration path (next slice).
//!
//! The reason probability lives here from day one is that retrofitting
//! it after a deterministic-only API has shipped is invasive: it changes
//! the engine's return type, the clause data shapes, and the indexing
//! strategy. Putting it in from the start, even when unused, keeps the
//! eventual extension purely additive.
//!
//! ## What's In This Slice
//!
//! - Probability, Fact, Rule, BodyLiteral, and FactId / RuleId.
//! - KnowledgeBase indexed by head functor/arity for clause lookup.
//! - SearchMode enum (with only FindFirst implemented for now).
//! - `find_first` — deterministic SLD-style resolution returning the
//!   first successful Substitution or None.
//! - Negation-as-failure inside rule bodies (Neg literals).
//!
//! Not in this slice: proof DAG construction, EnumerateAll / AutoDetect
//! mode implementations, weighted model counting.

pub mod enumerate;
pub mod proof_dag;
pub mod wmc;

use std::collections::HashMap;

use logic_core::{LogicVar, Substitution, Term, unify};

pub use enumerate::enumerate_all;
pub use proof_dag::{DerivationOrigin, Proof, ProofDAG, ProofStep};
pub use wmc::weighted_model_count;

// ---------------------------------------------------------------------------
// Identities and probability
// ---------------------------------------------------------------------------

/// Stable identifier for a Fact within a KnowledgeBase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FactId(pub u64);

/// Stable identifier for a Rule within a KnowledgeBase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RuleId(pub u64);

/// Probability annotation on a clause.
///
/// `Certain` is **not** sugar for `Value(1.0)`. It is a distinct variant
/// recognized structurally by the engine: programs that use only `Certain`
/// can short-circuit to deterministic find-first search without ever
/// constructing a proof DAG or invoking the weighted-model-counting
/// backend. The short-circuit detection itself lives in
/// [`KnowledgeBase::is_all_certain`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Probability {
    Certain,
    Value(f64),
}

impl Probability {
    /// The numeric probability of this annotation. `Certain` maps to 1.0.
    pub fn as_f64(&self) -> f64 {
        match self {
            Probability::Certain => 1.0,
            Probability::Value(p) => *p,
        }
    }

    /// `true` iff this annotation is genuinely uncertain — i.e., is a
    /// `Value(p)` with `p < 1.0 − ε`.
    pub fn is_uncertain(&self) -> bool {
        match self {
            Probability::Certain => false,
            Probability::Value(p) => *p < 1.0 - 1e-12,
        }
    }
}

// ---------------------------------------------------------------------------
// Clauses: Fact, Rule, BodyLiteral
// ---------------------------------------------------------------------------

/// A Fact is a clause with no body.
#[derive(Debug, Clone, PartialEq)]
pub struct Fact {
    pub id: FactId,
    pub term: Term,
    pub probability: Probability,
}

impl Fact {
    /// Construct a `Certain` Fact. The `id` is set when the Fact is
    /// added to a KnowledgeBase; for construction-time use, a sentinel
    /// id of `FactId(u64::MAX)` is assigned and overwritten on insert.
    pub fn certain(term: Term) -> Self {
        Self {
            id: FactId(u64::MAX),
            term,
            probability: Probability::Certain,
        }
    }

    /// Construct a Fact with explicit probability `p`. The `id` is set
    /// on insert (see `Fact::certain` for the sentinel rationale).
    pub fn with_probability(term: Term, p: f64) -> Self {
        Self {
            id: FactId(u64::MAX),
            term,
            probability: Probability::Value(p),
        }
    }
}

/// A literal inside a Rule's body.
#[derive(Debug, Clone, PartialEq)]
pub enum BodyLiteral {
    /// A positive subgoal: this term must be derivable.
    Pos(Term),
    /// Negation-as-failure: this term must **not** be derivable under
    /// the current substitution. Per LP19, negation in this engine is
    /// the well-founded reading; non-stratified programs are an error
    /// (statically detected once `LP19a`'s stratification check lands).
    Neg(Term),
}

/// A Rule has a head and a body.
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub id: RuleId,
    pub head: Term,
    pub body: Vec<BodyLiteral>,
    pub probability: Probability,
}

impl Rule {
    pub fn certain(head: Term, body: Vec<BodyLiteral>) -> Self {
        Self {
            id: RuleId(u64::MAX),
            head,
            body,
            probability: Probability::Certain,
        }
    }

    /// Construct a `Rule` with explicit probability `p`. Mirrors
    /// [`Fact::with_probability`]: the `id` is a sentinel
    /// `RuleId(u64::MAX)` that the KB overwrites on insert.
    pub fn with_probability(head: Term, body: Vec<BodyLiteral>, p: f64) -> Self {
        Self {
            id: RuleId(u64::MAX),
            head,
            body,
            probability: Probability::Value(p),
        }
    }
}

// ---------------------------------------------------------------------------
// Knowledge base — indexed by head functor/arity for fast clause lookup
// ---------------------------------------------------------------------------

/// Functor/arity index key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ClauseIndex {
    functor: String,
    arity: usize,
}

impl ClauseIndex {
    /// Extract a `(functor, arity)` index from a term, or `None` if the
    /// term is not a compound (in which case clause selection by
    /// functor/arity is not meaningful).
    fn from_term(term: &Term) -> Option<Self> {
        match term {
            Term::Compound { functor, args } => Some(Self {
                functor: functor.clone(),
                arity: args.len(),
            }),
            Term::Atom(name) => Some(Self {
                functor: name.clone(),
                arity: 0,
            }),
            _ => None,
        }
    }
}

/// A collection of Facts and Rules, indexed for clause selection by
/// the head's functor/arity.
#[derive(Debug, Default)]
pub struct KnowledgeBase {
    facts: HashMap<ClauseIndex, Vec<Fact>>,
    rules: HashMap<ClauseIndex, Vec<Rule>>,
    next_fact_id: u64,
    next_rule_id: u64,
}

impl KnowledgeBase {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a Fact, assigning it a fresh `FactId`.
    pub fn add_fact(&mut self, mut fact: Fact) -> FactId {
        let id = FactId(self.next_fact_id);
        self.next_fact_id += 1;
        fact.id = id;
        if let Some(idx) = ClauseIndex::from_term(&fact.term) {
            self.facts.entry(idx).or_default().push(fact);
        }
        id
    }

    /// Insert a Rule, assigning it a fresh `RuleId`.
    pub fn add_rule(&mut self, mut rule: Rule) -> RuleId {
        let id = RuleId(self.next_rule_id);
        self.next_rule_id += 1;
        rule.id = id;
        if let Some(idx) = ClauseIndex::from_term(&rule.head) {
            self.rules.entry(idx).or_default().push(rule);
        }
        id
    }

    pub(crate) fn facts_for(&self, term: &Term) -> &[Fact] {
        ClauseIndex::from_term(term)
            .and_then(|idx| self.facts.get(&idx))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub(crate) fn rules_for(&self, term: &Term) -> &[Rule] {
        ClauseIndex::from_term(term)
            .and_then(|idx| self.rules.get(&idx))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Look up a Fact by its `FactId`. O(N) over all facts; sufficient
    /// for current scale. Used by the weighted-model-counting backend
    /// when it needs to recover a probabilistic clause's parameter.
    pub fn find_fact_by_id(&self, id: FactId) -> Option<&Fact> {
        self.facts.values().flatten().find(|f| f.id == id)
    }

    /// Look up a Rule by its `RuleId`. O(N) over all rules.
    pub fn find_rule_by_id(&self, id: RuleId) -> Option<&Rule> {
        self.rules.values().flatten().find(|r| r.id == id)
    }

    /// Walk every Fact and every Rule once; return `true` iff every
    /// clause has `probability == Certain`. This is the precondition for
    /// the LP19 short-circuit.
    pub fn is_all_certain(&self) -> bool {
        let fact_certain = self
            .facts
            .values()
            .flat_map(|v| v.iter())
            .all(|f| f.probability == Probability::Certain);
        let rule_certain = self
            .rules
            .values()
            .flat_map(|v| v.iter())
            .all(|r| r.probability == Probability::Certain);
        fact_certain && rule_certain
    }
}

// ---------------------------------------------------------------------------
// Search modes (only FindFirst is implemented in this slice)
// ---------------------------------------------------------------------------

/// Per LP19, three search modes are defined. `FindFirst` stops at the
/// first successful derivation; `EnumerateAll` traverses every branch
/// and returns the complete proof DAG; `AutoDetect` chooses between
/// them based on whether the knowledge base is all-`Certain`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    FindFirst,
    EnumerateAll,
    AutoDetect,
}

/// What a search call returns. `FindFirstResult` is the cheap path —
/// at most one substitution. `EnumerateAllResult` carries the full proof
/// DAG along with the engine's computed probability for probabilistic
/// queries.
#[derive(Debug, Clone, PartialEq)]
pub enum SearchResult {
    /// Deterministic find-first: at most one binding, no DAG.
    FindFirstResult(Option<Substitution>),
    /// All-proofs enumeration with the (possibly trivial) WMC.
    EnumerateAllResult {
        dag: ProofDAG,
        /// `P(query)`. Equals 1.0 if every clause used is `Certain`
        /// and at least one proof exists; equals 0.0 if no proof.
        probability: f64,
    },
}

/// Run a query against the KB under the chosen search mode. When mode
/// is `AutoDetect`, the engine inspects `kb.is_all_certain()` and picks
/// `FindFirst` if every clause is `Certain`, otherwise `EnumerateAll`
/// — this is the LP19 short-circuit theorem made explicit.
pub fn search(query: &Term, kb: &KnowledgeBase, mode: SearchMode) -> SearchResult {
    let effective = match mode {
        SearchMode::FindFirst => SearchMode::FindFirst,
        SearchMode::EnumerateAll => SearchMode::EnumerateAll,
        SearchMode::AutoDetect => {
            if kb.is_all_certain() {
                SearchMode::FindFirst
            } else {
                SearchMode::EnumerateAll
            }
        }
    };

    match effective {
        SearchMode::FindFirst => SearchResult::FindFirstResult(find_first(query, kb)),
        SearchMode::EnumerateAll => {
            let dag = enumerate_all(query, kb);
            let probability = weighted_model_count(&dag, kb);
            SearchResult::EnumerateAllResult { dag, probability }
        }
        // AutoDetect was rewritten above.
        SearchMode::AutoDetect => unreachable!(),
    }
}

// ---------------------------------------------------------------------------
// Clause renaming — every clause is given fresh variables before unification
// ---------------------------------------------------------------------------

/// Walk `term` and replace every `Var` with a fresh variable, sharing
/// renames so that two occurrences of the same variable in the input map
/// to the same fresh variable in the output.
fn rename_term(term: &Term, renames: &mut HashMap<u64, LogicVar>) -> Term {
    match term {
        Term::Var(v) => {
            let fresh = renames
                .entry(v.id)
                .or_insert_with(|| {
                    LogicVar::fresh(v.display_name.as_deref())
                })
                .clone();
            Term::Var(fresh)
        }
        Term::Compound { functor, args } => Term::Compound {
            functor: functor.clone(),
            args: args.iter().map(|a| rename_term(a, renames)).collect(),
        },
        other => other.clone(),
    }
}

fn rename_literal(lit: &BodyLiteral, renames: &mut HashMap<u64, LogicVar>) -> BodyLiteral {
    match lit {
        BodyLiteral::Pos(t) => BodyLiteral::Pos(rename_term(t, renames)),
        BodyLiteral::Neg(t) => BodyLiteral::Neg(rename_term(t, renames)),
    }
}

// ---------------------------------------------------------------------------
// Deterministic find-first search (SLD-style, no proof DAG yet)
// ---------------------------------------------------------------------------

/// Find the first substitution that proves `query` against `kb`, or
/// `None` if no proof exists.
///
/// The algorithm is straightforward SLD resolution:
///
/// 1. Match `query` against every Fact whose functor/arity could possibly
///    unify with it. The first successful unification produces a result.
/// 2. Match `query` against every Rule whose head could possibly unify.
///    For each candidate rule:
///    - Rename the rule's variables to fresh ones (so that distinct
///      clause instances don't share variables).
///    - Unify `query` with the renamed head.
///    - If unification succeeds, recursively prove each body literal.
///      A `Pos(t)` literal succeeds iff `find_first(t, kb)` returns
///      Some; a `Neg(t)` literal succeeds iff `find_first(t, kb)`
///      returns None (negation-as-failure).
/// 3. Return the first substitution that makes all subgoals succeed.
///
/// Backtracking is implicit in the iteration: when a clause's body fails,
/// we drop the substitution and try the next clause.
pub fn find_first(query: &Term, kb: &KnowledgeBase) -> Option<Substitution> {
    find_first_with(query, kb, &Substitution::empty())
}

fn find_first_with(query: &Term, kb: &KnowledgeBase, subst: &Substitution) -> Option<Substitution> {
    let resolved = subst.walk(query);

    // Try facts first — they have no body, so success is immediate.
    for fact in kb.facts_for(&resolved) {
        let mut renames = HashMap::new();
        let renamed = rename_term(&fact.term, &mut renames);
        if let Some(s) = unify(&resolved, &renamed, subst) {
            return Some(s);
        }
    }

    // Then rules — try each candidate rule, unify head, prove body.
    for rule in kb.rules_for(&resolved) {
        let mut renames = HashMap::new();
        let renamed_head = rename_term(&rule.head, &mut renames);
        let renamed_body: Vec<BodyLiteral> = rule
            .body
            .iter()
            .map(|lit| rename_literal(lit, &mut renames))
            .collect();

        if let Some(mut s) = unify(&resolved, &renamed_head, subst) {
            if prove_body(&renamed_body, kb, &mut s) {
                return Some(s);
            }
        }
    }

    None
}

/// Prove every literal in `body` under the substitution `s`, threading
/// each successful subgoal's resulting substitution forward to the next.
fn prove_body(body: &[BodyLiteral], kb: &KnowledgeBase, s: &mut Substitution) -> bool {
    for literal in body {
        match literal {
            BodyLiteral::Pos(t) => match find_first_with(t, kb, s) {
                Some(next) => *s = next,
                None => return false,
            },
            BodyLiteral::Neg(t) => {
                if find_first_with(t, kb, s).is_some() {
                    // The negated goal is provable — negation-as-failure fails.
                    return false;
                }
                // Goal not provable; negation succeeds; substitution unchanged.
            }
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Inline unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use logic_core::{atom, compound, var, Term};

    fn empty_kb() -> KnowledgeBase {
        KnowledgeBase::new()
    }

    #[test]
    fn certain_probability_is_not_uncertain() {
        assert!(!Probability::Certain.is_uncertain());
        assert_eq!(Probability::Certain.as_f64(), 1.0);
    }

    #[test]
    fn value_at_one_is_certain_in_practice() {
        // Structurally distinct from Certain, but numerically equal.
        let p = Probability::Value(1.0);
        assert!(!p.is_uncertain());
        assert_eq!(p.as_f64(), 1.0);
        // But not structurally equal to Certain.
        assert_ne!(p, Probability::Certain);
    }

    #[test]
    fn value_below_one_is_uncertain() {
        assert!(Probability::Value(0.5).is_uncertain());
        assert!(Probability::Value(0.0).is_uncertain());
    }

    #[test]
    fn empty_kb_is_all_certain() {
        // Vacuously true: no clauses, no non-Certain probabilities.
        assert!(empty_kb().is_all_certain());
    }

    #[test]
    fn kb_with_all_certain_facts_is_all_certain() {
        let mut kb = empty_kb();
        kb.add_fact(Fact::certain(atom("a")));
        kb.add_fact(Fact::certain(atom("b")));
        assert!(kb.is_all_certain());
    }

    #[test]
    fn kb_with_one_probabilistic_fact_is_not_all_certain() {
        let mut kb = empty_kb();
        kb.add_fact(Fact::certain(atom("a")));
        kb.add_fact(Fact::with_probability(atom("b"), 0.7));
        assert!(!kb.is_all_certain());
    }

    #[test]
    fn find_first_returns_none_on_empty_kb() {
        let kb = empty_kb();
        assert!(find_first(&atom("nothing"), &kb).is_none());
    }

    #[test]
    fn find_first_matches_a_single_atom_fact() {
        let mut kb = empty_kb();
        kb.add_fact(Fact::certain(atom("homer")));
        let s = find_first(&atom("homer"), &kb).expect("should find the atom");
        // No variables in the query; substitution is empty.
        assert_eq!(s, Substitution::empty());
    }

    #[test]
    fn find_first_binds_a_variable_via_unification() {
        let mut kb = empty_kb();
        kb.add_fact(Fact::certain(compound(
            "father",
            vec![atom("homer"), atom("bart")],
        )));
        let x = var("X");
        let query = compound("father", vec![atom("homer"), Term::Var(x.clone())]);
        let s = find_first(&query, &kb).expect("father(homer, X) should succeed");
        assert_eq!(s.walk_var(&x), atom("bart"));
    }

    #[test]
    fn find_first_returns_first_matching_clause_on_backtrack_setup() {
        let mut kb = empty_kb();
        // Two facts; the first one listed should be tried first.
        kb.add_fact(Fact::certain(compound(
            "father",
            vec![atom("homer"), atom("bart")],
        )));
        kb.add_fact(Fact::certain(compound(
            "father",
            vec![atom("homer"), atom("lisa")],
        )));
        let x = var("X");
        let query = compound("father", vec![atom("homer"), Term::Var(x.clone())]);
        let s = find_first(&query, &kb).unwrap();
        assert_eq!(s.walk_var(&x), atom("bart"));
    }

    #[test]
    fn rule_with_one_body_literal_resolves_through_it() {
        // grandfather(X, Z) :- father(X, Y), father(Y, Z).
        // father(homer, bart).
        // father(grandpa, homer).
        let mut kb = empty_kb();
        kb.add_fact(Fact::certain(compound(
            "father",
            vec![atom("homer"), atom("bart")],
        )));
        kb.add_fact(Fact::certain(compound(
            "father",
            vec![atom("grandpa"), atom("homer")],
        )));

        let xx = var("X");
        let yy = var("Y");
        let zz = var("Z");
        kb.add_rule(Rule::certain(
            compound("grandfather", vec![Term::Var(xx.clone()), Term::Var(zz.clone())]),
            vec![
                BodyLiteral::Pos(compound(
                    "father",
                    vec![Term::Var(xx.clone()), Term::Var(yy.clone())],
                )),
                BodyLiteral::Pos(compound(
                    "father",
                    vec![Term::Var(yy.clone()), Term::Var(zz.clone())],
                )),
            ],
        ));

        // Query: grandfather(grandpa, Who).
        let who = var("Who");
        let query = compound(
            "grandfather",
            vec![atom("grandpa"), Term::Var(who.clone())],
        );
        let s = find_first(&query, &kb).expect("grandfather(grandpa, Who) should succeed");
        assert_eq!(s.walk_var(&who), atom("bart"));
    }

    #[test]
    fn negation_as_failure_succeeds_when_goal_is_not_provable() {
        // q(X) :- \+ p(X).   (q(X) succeeds iff p(X) cannot be proved)
        // p(a).
        let mut kb = empty_kb();
        kb.add_fact(Fact::certain(compound("p", vec![atom("a")])));

        let x = var("X");
        kb.add_rule(Rule::certain(
            compound("q", vec![Term::Var(x.clone())]),
            vec![BodyLiteral::Neg(compound("p", vec![Term::Var(x.clone())]))],
        ));

        // q(b) succeeds because p(b) is not provable.
        let q_b = compound("q", vec![atom("b")]);
        assert!(find_first(&q_b, &kb).is_some());

        // q(a) fails because p(a) is provable.
        let q_a = compound("q", vec![atom("a")]);
        assert!(find_first(&q_a, &kb).is_none());
    }

    #[test]
    fn rules_index_does_not_match_unrelated_functor() {
        let mut kb = empty_kb();
        kb.add_fact(Fact::certain(compound("p", vec![atom("x")])));
        // Query for q/1 — must not match any p/1 fact via the index.
        assert!(find_first(&compound("q", vec![atom("x")]), &kb).is_none());
    }

    #[test]
    fn fresh_variable_renaming_avoids_collision_across_clause_uses() {
        // p(X) :- q(X, X).
        // q(a, a).  q(b, c).
        // ?- p(W).  W should be bound to 'a' (the only X that satisfies q(X, X))
        let mut kb = empty_kb();
        kb.add_fact(Fact::certain(compound(
            "q",
            vec![atom("a"), atom("a")],
        )));
        kb.add_fact(Fact::certain(compound(
            "q",
            vec![atom("b"), atom("c")],
        )));

        let x = var("X");
        kb.add_rule(Rule::certain(
            compound("p", vec![Term::Var(x.clone())]),
            vec![BodyLiteral::Pos(compound(
                "q",
                vec![Term::Var(x.clone()), Term::Var(x.clone())],
            ))],
        ));

        let w = var("W");
        let s = find_first(&compound("p", vec![Term::Var(w.clone())]), &kb)
            .expect("p(W) should succeed via q(a, a)");
        assert_eq!(s.walk_var(&w), atom("a"));
    }
}
