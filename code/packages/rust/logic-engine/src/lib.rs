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

pub mod compute;
pub mod conversion;
pub mod datetime;
pub mod differential;
pub mod dimension;
pub mod enumerate;
pub mod govern;
pub mod lr_aggregate;
pub mod proof_dag;
pub mod provenance;
pub mod verify;
pub mod wmc;

use std::collections::{HashMap, HashSet};

use logic_core::{unify, LogicVar, Number, Substitution, Term};

pub use compute::{
    compute, ComputeError, ComputeExpr, ComputeOp, DerivationNode, Derived, RoundSpec,
};
/// Re-exported so consumers can name the rounding mode of a
/// [`ComputeExpr::Round`]/[`DerivationNode::Round`] without depending on
/// `bignum-core` directly (NUM-6a).
pub use bignum_core::RoundingMode;
pub use conversion::{add_or_sub, convert_value, ConvError, Conversion, ConversionTable};
pub use datetime::{
    after, before, date_add, date_ordinal, days_between, read_date, read_duration_days,
};
pub use differential::{differential, Differential, DifferentialDecision, RankedHypothesis};
pub use dimension::{dimensioned_value, DimError, DimOp, Dimension, Dimensioned};
pub use enumerate::{enumerate_all, ResolutionLimitExceeded, MAX_SLD_DEPTH};
pub use govern::{
    enumerate_governing, ConflictStatus, GovernStatus, GovernedAnswer, GovernedResult,
};
pub use lr_aggregate::{
    counterfactual, lr_aggregate, sigmoid, source_disagreements,
    source_disagreements_with_threshold, CmpOp, ContributionClause, JointContributionClause,
    KbError, KickbackReport, LRAggregateResult, LrAggregateWarning, PredicateContributionClause,
    PriorClause, SourceDisagreementReport, SourceLogitDelta, UncertaintyMarker, UncertaintyReport,
};
pub use proof_dag::{DerivationOrigin, Proof, ProofDAG, ProofStep};
pub use provenance::{Citation, ContentHash, Provenance, Quote, TrustTier, VerbatimSpan};
pub use verify::{
    verify_proof, verify_quote, verify_step, LogicFailure, LogicStatus, MemorySnapshots,
    NoSnapshots, QuoteMiss, QuoteStatus, SnapshotStore, StepVerification, TraceVerification,
    UnverifiedReason,
};
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

/// Stable identifier for a [`PriorClause`] within a KnowledgeBase.
///
/// Distinct id type from `FactId` / `RuleId` so the proof DAG and
/// the `DerivationOrigin` enum can statically distinguish what kind
/// of clause a step came from. The lowering map (ADJ15) joins these
/// ids back to the user-facing rulebook line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PriorClauseId(pub u64);

/// Stable identifier for a [`ContributionClause`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContributionClauseId(pub u64);

/// Stable identifier for a [`JointContributionClause`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct JointContributionClauseId(pub u64);

/// Stable identifier for a
/// [`PredicateContributionClause`](crate::lr_aggregate::PredicateContributionClause).
///
/// Predicate-gated contributions are the deterministic-as-saturating-
/// probabilistic bridge (ADJ "deterministic = special case"): a numeric
/// predicate over a valued slot, evaluated on CPU, gates a `logit_delta`.
/// Its own id type keeps the proof DAG able to distinguish a fired
/// predicate from an ordinary single-source contribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PredicateContributionClauseId(pub u64);

/// Stable identifier for an
/// [`UncertaintyMarker`](crate::lr_aggregate::UncertaintyMarker).
///
/// LP19e + ADJ47-D: uncertainty markers are the engine-layer
/// representation of "the patient (or the source) did not specify
/// this value, but we know the domain it ranges over." The
/// LR-aggregation result surfaces an
/// [`UncertaintyReport`](crate::lr_aggregate::UncertaintyReport) for
/// every active marker whose domain has zero observed members — so
/// the audit reader can see *what would shift the answer* without
/// having to look it up themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UncertaintyMarkerId(pub u64);

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
    /// The citation for this fact — **mandatory**: every fact is accountable.
    /// Ground *relational edges* (the `relate deficient_in(tay_sachs,
    /// hexosaminidase_a)` surface form) carry a real [`Provenance`] so a binding
    /// query's answer can be returned WITH a proof — the byte-provenanced source
    /// that justifies the edge. Ordinary `observe`d facts (whose justification
    /// lives in the clauses that read them) carry [`Provenance::unattributed`],
    /// the explicit "no source" value rather than a silent `None`.
    pub provenance: Provenance,
}

impl Fact {
    /// Construct a `Certain` Fact. The `id` is set when the Fact is
    /// added to a KnowledgeBase; for construction-time use, a sentinel
    /// id of `FactId(u64::MAX)` is assigned and overwritten on insert.
    /// Provenance defaults to [`Provenance::unattributed`] — attach a real
    /// citation with [`Fact::with_provenance`].
    pub fn certain(term: Term) -> Self {
        Self {
            id: FactId(u64::MAX),
            term,
            probability: Probability::Certain,
            provenance: Provenance::unattributed(),
        }
    }

    /// Construct a Fact with explicit probability `p`. The `id` is set
    /// on insert (see `Fact::certain` for the sentinel rationale).
    pub fn with_probability(term: Term, p: f64) -> Self {
        Self {
            id: FactId(u64::MAX),
            term,
            probability: Probability::Value(p),
            provenance: Provenance::unattributed(),
        }
    }

    /// Attach a citation to this fact (builder). Used by the lowerer to carry a
    /// `relate` edge's `source`/`locator`/`trust` annotations onto the Fact, so
    /// the edge that answers a binding query can be cited.
    pub fn with_provenance(mut self, provenance: Provenance) -> Self {
        self.provenance = provenance;
        self
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
    /// Where this rule came from — a byte-quoted citation when the rule was
    /// grounded from source text (a payer policy, an FDA label, a guideline) and
    /// gated into the CAS. Defaults to [`Provenance::unattributed`] for rules with
    /// no citation. Mirrors [`Fact::provenance`], so a derived consequence can (in
    /// future) cite the rule that produced it alongside the facts it fired on.
    pub provenance: Provenance,
    /// ADJ73 defeasible precedence: the rule's priority TIER among *conflicting*
    /// derivations. A higher tier defeats a lower one when two rules derive heads that
    /// cannot both hold (a predicate declared functional via
    /// [`KnowledgeBase::declare_functional`]). Defaults to [`Priority::Default`]; a plain
    /// rulebook with no priorities and no functional predicates behaves exactly as before
    /// (every conclusion governs). Only [`crate::govern::enumerate_governing`] reads this
    /// field — `enumerate_all` ignores it, so monotonic queries are unchanged.
    ///
    /// Named TIERS, not raw integers (ADJ73 decision 1): the explicit tier is the simplest
    /// grounded precedence principle ("a higher tier wins"), used for local default-vs-
    /// exception ladders. Richer, byte-provenanced precedence (lex-superior / recency /
    /// appeal-status over a grounded `context-precedence` rulebook) is ADJ73 PR-B.
    pub priority: Priority,
    /// ADJ73 PR-B (grounded context precedence): the CONTEXT this rule is grounded in — a
    /// jurisdiction (`ninth_circuit`), guideline edition (`idsa_2024`), specialty
    /// (`specialist`), etc. `None` for a context-free rule (today's behavior). When two
    /// conflicting rules carry contexts ordered by [`KnowledgeBase::add_context_outranks`]
    /// (e.g. `ninth_circuit` outranks `district_court`), the rule in the GREATER context
    /// defeats the other — the McCarthy lex-superior relation — *before* the priority tier is
    /// consulted (the tier breaks ties the context order leaves open). Only
    /// [`crate::govern::enumerate_governing`] reads it.
    pub context: Option<String>,
}

/// ADJ73 defeasible-precedence TIER (decision 1: named enum, not raw integers). Totally
/// ordered lowest→highest by declaration order, so `derive(Ord)` gives `Default < Specific
/// < Authoritative < Mandatory`. A ground **fact** sits above every tier (asserted truth);
/// that "above all" standing is represented by [`crate::govern::Standing`], not here.
///
/// - `Default` — the implicit fallback rule (what a rule has when no tier is written).
/// - `Specific` — a more specific rule than the general default.
/// - `Authoritative` — a rule from a governing/authoritative source.
/// - `Mandatory` — a hard override (the highest rule tier).
///
/// These are domain-neutral: a clinical "specialist guideline over general" and a legal
/// "controlling authority over persuasive" both map onto the same ladder. The names are
/// open to revision (ADJ73 §2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Priority {
    #[default]
    Default,
    Specific,
    Authoritative,
    Mandatory,
}

impl Rule {
    pub fn certain(head: Term, body: Vec<BodyLiteral>) -> Self {
        Self {
            id: RuleId(u64::MAX),
            head,
            body,
            probability: Probability::Certain,
            provenance: Provenance::unattributed(),
            priority: Priority::Default,
            context: None,
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
            provenance: Provenance::unattributed(),
            priority: Priority::Default,
            context: None,
        }
    }

    /// Attach a citation (mirrors [`Fact::with_provenance`]) — used when a rule is
    /// grounded from source text and gated into the CAS.
    pub fn with_provenance(mut self, provenance: Provenance) -> Self {
        self.provenance = provenance;
        self
    }

    /// ADJ73: set the rule's defeasible-precedence priority (higher defeats lower
    /// among conflicting derivations). Builder-style, mirrors [`Self::with_provenance`].
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// ADJ73 PR-B: ground the rule in a CONTEXT (a jurisdiction / guideline edition /
    /// specialty). Combined with [`KnowledgeBase::add_context_outranks`], the rule in the
    /// greater context defeats a conflicting one in a lesser context (lex superior).
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
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

/// Evidence that can gate an LR contribution.
///
/// A direct `observe e` remains the common fast path: `fact_ids` names the
/// observed fact(s), `proof` is absent, and `confidence` is exactly 1.0. When
/// `e` is only derivable through rules, `proof` carries the SLD derivation and
/// `confidence` is the product of the probabilities on the facts/rules used by
/// that proof. LR aggregation uses that confidence to attenuate the
/// contribution instead of forcing a brittle all-or-nothing bridge.
#[derive(Debug, Clone, PartialEq)]
pub struct ObservedEvidence {
    pub fact_ids: Vec<FactId>,
    pub rule_ids: Vec<RuleId>,
    pub proof: Option<Box<Proof>>,
    pub confidence: f64,
}

/// A collection of Facts and Rules, indexed for clause selection by
/// the head's functor/arity. Per LP19e, also stores prior /
/// contribution / joint-contribution clauses in parallel maps; the
/// SLD-resolution / WMC paths ignore these and the LR-aggregation
/// path ignores `facts` and `rules` except via the `observed_evidence`
/// query — the two inference shapes coexist without interference.
///
/// **Cloneable** (ADJ47-E): so counterfactual queries can be served
/// by cloning the KB, mutating the clone, and rerunning aggregation
/// without disturbing the caller's KB.
#[derive(Debug, Default, Clone)]
pub struct KnowledgeBase {
    facts: HashMap<ClauseIndex, Vec<Fact>>,
    rules: HashMap<ClauseIndex, Vec<Rule>>,
    /// LP19e: at most one PriorClause per conclusion. Stored as a
    /// flat `Vec` rather than a `HashMap<Term, _>` because `Term` does
    /// not implement `Hash + Eq` (it embeds `LogicVar` and `f64`s
    /// nowhere yet, but the contract may grow). Linear scan is fine
    /// at the scale we use today (≤ ~50 priors per KB even for a
    /// medical rulebook); switching to an indexed map later is purely
    /// additive once `Term: Hash + Eq` is available.
    priors: Vec<PriorClause>,
    contributions: Vec<ContributionClause>,
    joint_contributions: Vec<JointContributionClause>,
    /// Predicate-gated contributions (deterministic = saturating
    /// probabilistic). Each fires when the observed numeric value of its
    /// slot satisfies a CPU-evaluated comparison.
    predicate_contributions: Vec<PredicateContributionClause>,
    /// Derived values bound by `let name = expr` (ADJ expansion step 3).
    /// Each carries its [`compute::Derived`] derivation tree.
    /// [`observed_value`](Self::observed_value) falls back to this table so a
    /// predicate fires over a computed value exactly as over an observed one.
    derived: Vec<crate::compute::Derived>,
    /// LP19e + ADJ47-D: uncertainty markers attached to conclusions.
    /// Each marker carries a domain of candidate evidence terms; if
    /// none of them is observed, the aggregator emits an
    /// [`UncertaintyReport`] in the result so the audit reader sees
    /// what's missing.
    uncertainty_markers: Vec<UncertaintyMarker>,
    /// ADJ73 defeasible precedence: the set of predicates (by functor/arity) that are
    /// FUNCTIONAL on their last argument — at most one value may hold per key (the
    /// preceding args). Two derivations that agree on the key but differ on the last
    /// argument *conflict*, and [`crate::govern::enumerate_governing`] keeps only the
    /// highest-priority one. A predicate not listed here is monotonic (every derivation
    /// governs) — this is what makes precedence opt-in and `enumerate_all` unchanged.
    functional_predicates: HashSet<ClauseIndex>,
    /// ADJ73 PR-B: the EXPLICIT half of the CONTEXT precedence order — directed edges
    /// `(higher, lower)` meaning "a rule in `higher` outranks a conflicting rule in `lower`"
    /// (federal > state, ninth_circuit > district_court, idsa_2024 > idsa_2004,
    /// specialist > general), declared via [`Self::add_context_outranks`] (the bare
    /// `context_order { a > b }` surface form). These edges carry NO provenance. The GROUNDED
    /// half — edges that DO carry a byte-quote (the Supremacy Clause, etc.) — lives as ordinary
    /// `outranks_context(higher, lower)` facts in the fact store; [`Self::context_adjacency`] unions
    /// the two so both feed [`crate::govern::enumerate_governing`], which consults the order
    /// BEFORE the priority tier (lex superior). Transitive reach is computed cycle-safely.
    context_order: Vec<(String, String)>,
    next_fact_id: u64,
    next_rule_id: u64,
    next_prior_id: u64,
    next_contribution_id: u64,
    next_joint_contribution_id: u64,
    next_predicate_contribution_id: u64,
    next_uncertainty_marker_id: u64,
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

    /// Look up a Fact by its `FactId`. Used to resolve a proof's `via_facts` (or
    /// a `DerivationOrigin::FromFact`) back to the firing fact — in particular its
    /// [`Fact::provenance`], so a binding query's answer can be returned WITH the
    /// citing edge's source. Facts are bucketed by clause index, so this scans the
    /// (small) fact store; callers needing it in a hot loop should cache.
    pub fn fact(&self, id: FactId) -> Option<&Fact> {
        self.facts.values().flatten().find(|f| f.id == id)
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

    /// ADJ73: declare a predicate FUNCTIONAL on its last argument — at most one value
    /// may hold per key (the preceding arguments). E.g. `declare_functional("timing", 1)`
    /// makes every `timing(_)` derivation conflict (key is empty), so precedence picks one.
    /// Idempotent. A predicate that is never declared functional stays monotonic.
    pub fn declare_functional(&mut self, functor: &str, arity: usize) {
        self.functional_predicates.insert(ClauseIndex {
            functor: functor.to_string(),
            arity,
        });
    }

    /// ADJ73: is this term's predicate functional on its last argument? (Used by
    /// [`crate::govern::enumerate_governing`] to decide which answers can conflict.)
    pub(crate) fn is_functional(&self, term: &Term) -> bool {
        ClauseIndex::from_term(term)
            .map(|idx| self.functional_predicates.contains(&idx))
            .unwrap_or(false)
    }

    /// ADJ73 PR-B: assert that context `higher` OUTRANKS context `lower` (a grounded
    /// precedence edge — federal > state, ninth_circuit > district_court). Idempotent; the
    /// transitive closure is computed on query. Adding a back-edge that creates a cycle is
    /// *allowed* here (the loader may reject it), but [`Self::context_outranks`] is cycle-safe
    /// and a mutual outrank degrades to an unresolved conflict rather than a wrong pick.
    pub fn add_context_outranks(&mut self, higher: impl Into<String>, lower: impl Into<String>) {
        let edge = (higher.into(), lower.into());
        if !self.context_order.contains(&edge) {
            self.context_order.push(edge);
        }
    }

    /// ADJ73 PR-B: the EFFECTIVE context-precedence edges — the union of two sources, so an edge
    /// may be declared either way and both participate in the same `lex superior` resolution:
    ///
    ///  1. **Explicit** edges from [`Self::add_context_outranks`] (the bare `context_order { a > b }`
    ///     surface form) — convenient but unprovenanced.
    ///  2. **Grounded** edges from any ground fact `outranks_context(higher, lower)` — the
    ///     byte-provenanced form. A `relate outranks_context(federal, state) source "…Supremacy
    ///     Clause…" trust authoritative` clause is an ordinary [`Fact`] (queryable, CAS-correctable,
    ///     carrying [`Fact::provenance`]) that ALSO acts as a precedence edge. This is the whole
    ///     point of ADJ73's "context must be grounded": the *reason* federal outranks state is the
    ///     cited clause, riding on the edge itself rather than asserted bare in host code.
    ///
    /// Both args of the grounding fact must be atoms; a fact with a variable or nested compound arg
    /// is not a ground edge and is ignored here (it stays a normal queryable fact). Returns the
    /// edges as a directed ADJACENCY MAP `higher → [lowers]` so a graph walk does a single O(1)
    /// neighbour lookup per node instead of re-scanning the whole edge list (the context order is
    /// meant to scale to large rule corpora — e.g. a jurisdiction graph over the US Code — so the
    /// walks below stay O(V+E), not O(V·E)).
    ///
    /// ADJ73 PR-B-4 — DERIVED edges. When the KB contains RULES whose head is `outranks_context/2`
    /// (the grounded conflict-resolution META-RULES: lex posterior / lex specialis / appeal status),
    /// the precedence order is no longer just hand-asserted facts — it is *derived* from more
    /// primitive grounded facts (`supersedes`, `reverses`, …) via those rules. In that case we
    /// enumerate every provable `outranks_context($A, $B)` (which subsumes the ground facts, since a
    /// fact is a one-step derivation) so a meta-rule edge participates in `lex superior` exactly like
    /// an asserted one. With no such rules (the common case) we keep the cheap ground-fact scan.
    /// Returns OWNED strings because a derived answer term is built during enumeration, not borrowed
    /// from `self`.
    fn context_adjacency(&self) -> HashMap<String, Vec<String>> {
        let mut adj: HashMap<String, Vec<String>> = HashMap::new();
        // 1. Explicit edges (the bare `context_order { a > b }` surface form).
        for (hi, lo) in &self.context_order {
            adj.entry(hi.clone()).or_default().push(lo.clone());
        }
        // 2. Grounded + DERIVED edges from the `outranks_context/2` relation.
        let outranks_idx = ClauseIndex {
            functor: "outranks_context".to_string(),
            arity: 2,
        };
        if self.rules.contains_key(&outranks_idx) {
            // Meta-rules can DERIVE precedence → enumerate every provable edge (this already
            // includes the ground facts, each a one-step proof).
            for (hi, lo) in self.derived_context_edges() {
                adj.entry(hi).or_default().push(lo);
            }
        } else {
            // No meta-rules → the cheap path: ground `outranks_context(hi, lo)` facts (both atoms).
            for fact in self.facts.values().flatten() {
                if let Term::Compound { functor, args } = &fact.term {
                    if functor == "outranks_context" && args.len() == 2 {
                        if let (Term::Atom(hi), Term::Atom(lo)) = (&args[0], &args[1]) {
                            adj.entry(hi.clone()).or_default().push(lo.clone());
                        }
                    }
                }
            }
        }
        adj
    }

    /// ADJ73 PR-B-4: enumerate every provable `outranks_context($A, $B)` answer whose two arguments
    /// resolve to atoms, returning the `(higher, lower)` ground edges. This is what lets a grounded
    /// META-RULE (`rule { head: outranks_context($H, $L) when: reverses($H, $L) }`, itself citing the
    /// canon) contribute precedence edges derived from primitive grounded facts. Pure read over the
    /// KB (no mutation); not re-entrant with `enumerate_governing` because it queries a *different*
    /// predicate and never consults the context order itself.
    fn derived_context_edges(&self) -> Vec<(String, String)> {
        let a = LogicVar::fresh(Some("A"));
        let b = LogicVar::fresh(Some("B"));
        let query = Term::Compound {
            functor: "outranks_context".to_string(),
            args: vec![Term::Var(a.clone()), Term::Var(b.clone())],
        };
        let dag = enumerate::enumerate_all(&query, self);
        let mut edges = Vec::new();
        for proof in &dag.proofs {
            if let (Term::Atom(hi), Term::Atom(lo)) = (
                proof.bindings.walk(&Term::Var(a.clone())),
                proof.bindings.walk(&Term::Var(b.clone())),
            ) {
                edges.push((hi, lo));
            }
        }
        edges
    }

    /// ADJ73 PR-B: does context `a` outrank context `b` (directly or transitively)? Cycle-safe
    /// DFS over the effective context adjacency (explicit + grounded/derived edges, see
    /// [`Self::context_adjacency`]). `a == b` is `false` (a context does not outrank itself).
    /// Returns `false` when there is no directed path `a → … → b`.
    pub fn context_outranks(&self, a: &str, b: &str) -> bool {
        if a == b {
            return false;
        }
        let adj = self.context_adjacency();
        let mut stack: Vec<&str> = vec![a];
        let mut seen: HashSet<&str> = HashSet::new();
        while let Some(node) = stack.pop() {
            if !seen.insert(node) {
                continue; // already visited — cycle-safe
            }
            if let Some(neighbours) = adj.get(node) {
                for lo in neighbours {
                    if lo == b {
                        return true;
                    }
                    stack.push(lo.as_str());
                }
            }
        }
        false
    }

    /// ADJ73 PR-B: `true` iff the effective context order (explicit + grounded/derived edges, see
    /// [`Self::context_adjacency`]) contains a cycle (e.g. `a > b`, `b > a`, or a self-loop). The
    /// surface/loader should reject such a rulebook; the resolver itself stays safe regardless.
    /// Catches a cycle formed *across* the edge sources too (an explicit `a > b` plus a grounded or
    /// meta-rule-derived `outranks_context(b, a)`). Single Kahn topological-sort pass: a directed
    /// acyclic graph fully drains to zero in-degree, so any node left unremoved lies on a cycle.
    pub fn context_order_has_cycle(&self) -> bool {
        let adj = self.context_adjacency();
        // In-degree of every node that appears as a head or a tail.
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        for (hi, lowers) in &adj {
            in_degree.entry(hi.as_str()).or_insert(0);
            for lo in lowers {
                *in_degree.entry(lo.as_str()).or_insert(0) += 1;
            }
        }
        // Kahn: repeatedly retire a zero-in-degree node, decrementing its successors.
        let mut ready: Vec<&str> = in_degree
            .iter()
            .filter(|&(_, &d)| d == 0)
            .map(|(&n, _)| n)
            .collect();
        let mut retired = 0usize;
        while let Some(node) = ready.pop() {
            retired += 1;
            if let Some(neighbours) = adj.get(node) {
                for lo in neighbours {
                    let d = in_degree
                        .get_mut(lo.as_str())
                        .expect("successor was counted above");
                    *d -= 1;
                    if *d == 0 {
                        ready.push(lo.as_str());
                    }
                }
            }
        }
        // Anything not retired sits on (or downstream of) a cycle.
        retired != in_degree.len()
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

    // -----------------------------------------------------------------
    // LP19e — prior / contribution / joint-contribution storage
    // -----------------------------------------------------------------

    /// Insert a [`PriorClause`], assigning it a fresh
    /// [`PriorClauseId`]. Fails with [`KbError::ConflictingPriors`]
    /// if a prior for the same conclusion already exists.
    pub fn add_prior(&mut self, mut clause: PriorClause) -> Result<PriorClauseId, KbError> {
        if let Some(existing) = self
            .priors
            .iter()
            .find(|p| p.conclusion == clause.conclusion)
        {
            return Err(KbError::ConflictingPriors {
                conclusion: clause.conclusion,
                existing: existing.id,
            });
        }
        let id = PriorClauseId(self.next_prior_id);
        self.next_prior_id += 1;
        clause.id = id;
        self.priors.push(clause);
        Ok(id)
    }

    /// Insert a [`ContributionClause`], assigning a fresh id. Multiple
    /// contributions per `(conclusion, evidence_term)` are permitted
    /// and sum in log-odds at aggregation time.
    pub fn add_contribution(&mut self, mut clause: ContributionClause) -> ContributionClauseId {
        let id = ContributionClauseId(self.next_contribution_id);
        self.next_contribution_id += 1;
        clause.id = id;
        self.contributions.push(clause);
        id
    }

    /// Insert a [`JointContributionClause`].
    pub fn add_joint_contribution(
        &mut self,
        mut clause: JointContributionClause,
    ) -> JointContributionClauseId {
        let id = JointContributionClauseId(self.next_joint_contribution_id);
        self.next_joint_contribution_id += 1;
        clause.id = id;
        self.joint_contributions.push(clause);
        id
    }

    /// Insert a [`PredicateContributionClause`], assigning a fresh id.
    /// A predicate-gated contribution is the deterministic-as-saturating-
    /// probabilistic bridge: it fires when the observed numeric value of
    /// `slot` satisfies the clause's comparison, contributing its
    /// `logit_delta`. Multiple per conclusion are permitted and sum.
    pub fn add_predicate_contribution(
        &mut self,
        mut clause: PredicateContributionClause,
    ) -> PredicateContributionClauseId {
        let id = PredicateContributionClauseId(self.next_predicate_contribution_id);
        self.next_predicate_contribution_id += 1;
        clause.id = id;
        self.predicate_contributions.push(clause);
        id
    }

    /// Look up the unique prior on `conclusion`, if any. O(N) over
    /// all priors in the KB; see the `priors` field doc for why the
    /// flat-Vec representation is fine at current scale.
    pub fn prior_for(&self, conclusion: &Term) -> Option<&PriorClause> {
        self.priors.iter().find(|p| &p.conclusion == conclusion)
    }

    /// Iterate the single-source contributions naming `conclusion`.
    /// O(N) filter — small in practice.
    pub fn contributions_for(&self, conclusion: &Term) -> Vec<&ContributionClause> {
        self.contributions
            .iter()
            .filter(|c| &c.conclusion == conclusion)
            .collect()
    }

    /// Iterate the joint contributions naming `conclusion`.
    pub fn joint_contributions_for(&self, conclusion: &Term) -> Vec<&JointContributionClause> {
        self.joint_contributions
            .iter()
            .filter(|c| &c.conclusion == conclusion)
            .collect()
    }

    /// Iterate the predicate-gated contributions naming `conclusion`.
    pub fn predicate_contributions_for(
        &self,
        conclusion: &Term,
    ) -> Vec<&PredicateContributionClause> {
        self.predicate_contributions
            .iter()
            .filter(|c| &c.conclusion == conclusion)
            .collect()
    }

    /// Read the observed numeric value of a valued slot, if one was
    /// observed. A valued fact has the shape `slot(V)` — for example
    /// `observe gross_income(18000)` stores
    /// `Compound { functor: "gross_income", args: [Num(Int(18000))] }`.
    ///
    /// `V` may be either a bare number **or a typed-value wrapper** that
    /// carries the magnitude first: `quantity(18000, usd)`,
    /// `money(18000, usd)`, `percentage(40)`, `duration(365, days)`,
    /// `count(3)`. The magnitude is the wrapper's leading numeric
    /// argument — this is the unit-bearing typed value the ADJ language
    /// expansion (step 2) extracts, and it lets a predicate
    /// `gross_income >= 14600` fire over `quantity(18000, usd)` while the
    /// `usd` unit travels with the fact for the faithfulness gate. See
    /// [`numeric_magnitude`].
    ///
    /// Only `Certain` facts gate predicates (same scope as
    /// [`observed_evidence`](Self::observed_evidence)). Returns the value
    /// of the most-recently-added matching fact, as `f64`.
    pub fn observed_value(&self, slot: &str) -> Option<f64> {
        // An observed fact wins; otherwise fall back to a `let`-bound derived
        // value (ADJ expansion step 3) so a predicate fires over a computed
        // value exactly as it would over an observed one.
        self.observed_value_with_fact(slot)
            .map(|(v, _)| v)
            .or_else(|| self.derived_for(slot).map(|d| d.value))
    }

    /// Read an observed or derived numeric value together with its exact
    /// rational sidecar when one is available. This is used by equality-heavy
    /// predicate gates such as `answer == 3 / 10`: the public magnitude remains
    /// `f64`, but exact integer/rational arithmetic can avoid float artifacts.
    pub fn observed_numeric(
        &self,
        slot: &str,
    ) -> Option<(f64, Option<crate::compute::ExactRational>)> {
        self.observed_value_with_fact(slot)
            .map(|(v, id)| {
                let exact = self
                    .observed_exact_value_with_fact(slot)
                    .and_then(|(x, exact_id)| if exact_id == id { Some(x) } else { None });
                (v, exact)
            })
            .or_else(|| self.derived_for(slot).map(|d| (d.value, d.exact.clone())))
    }

    /// Like [`observed_value`](Self::observed_value) but also returns the
    /// [`FactId`] of the winning observation — used by
    /// [`compute`](crate::compute) so a derivation-tree leaf can cite the byte-
    /// grounded fact it came from. Does **not** consult the derived table
    /// (a derived value has no `FactId`; the caller records a `DerivedRef`).
    pub fn observed_value_with_fact(&self, slot: &str) -> Option<(f64, FactId)> {
        self.facts
            .values()
            .flatten()
            .filter(|f| f.probability == Probability::Certain)
            .filter_map(|f| match &f.term {
                Term::Compound { functor, args } if functor == slot && args.len() == 1 => {
                    numeric_magnitude(&args[0]).map(|v| (f.id, v))
                }
                _ => None,
            })
            // Largest FactId wins — facts are inserted in program order,
            // so a later `observe` of the same slot supersedes an earlier.
            .max_by_key(|(id, _)| id.0)
            .map(|(id, v)| (v, id))
    }

    /// Exact counterpart to [`observed_value_with_fact`](Self::observed_value_with_fact).
    /// Returns a value only when the fact's leading magnitude is exactly
    /// representable as an integer/rational sidecar.
    pub fn observed_exact_value_with_fact(
        &self,
        slot: &str,
    ) -> Option<(crate::compute::ExactRational, FactId)> {
        self.facts
            .values()
            .flatten()
            .filter(|f| f.probability == Probability::Certain)
            .filter_map(|f| match &f.term {
                Term::Compound { functor, args } if functor == slot && args.len() == 1 => {
                    numeric_exact_magnitude(&args[0]).map(|v| (f.id, v))
                }
                _ => None,
            })
            .max_by_key(|(id, _)| id.0)
            .map(|(id, v)| (v, id))
    }

    /// The observed **dimensioned** value of a slot (magnitude + its
    /// [`Dimension`](crate::Dimension)) with its [`FactId`]. Same
    /// latest-observation-wins rule as [`observed_value_with_fact`], but reads
    /// the unit/currency too, so the faithfulness gate (track A4) can reject
    /// `usd + days`. Returns `None` for a date/time term (those have no scalar
    /// magnitude — see [`dimensioned_value`](crate::dimensioned_value)).
    pub fn observed_dimensioned(&self, slot: &str) -> Option<(crate::Dimensioned, FactId)> {
        self.facts
            .values()
            .flatten()
            .filter(|f| f.probability == Probability::Certain)
            .filter_map(|f| match &f.term {
                Term::Compound { functor, args } if functor == slot && args.len() == 1 => {
                    crate::dimensioned_value(&args[0]).map(|d| (f.id, d))
                }
                _ => None,
            })
            .max_by_key(|(id, _)| id.0)
            .map(|(id, d)| (d, id))
    }

    /// Every observed numeric value of a slot, in fact-insertion order, with
    /// each value's [`FactId`]. This is what aggregations (`sum`/`count`/…)
    /// reduce — each observation becomes a cited leaf in the derivation tree.
    pub fn observed_values_all(&self, slot: &str) -> Vec<(f64, FactId)> {
        let mut out: Vec<(f64, FactId)> = self
            .facts
            .values()
            .flatten()
            .filter(|f| f.probability == Probability::Certain)
            .filter_map(|f| match &f.term {
                Term::Compound { functor, args } if functor == slot && args.len() == 1 => {
                    numeric_magnitude(&args[0]).map(|v| (v, f.id))
                }
                _ => None,
            })
            .collect();
        out.sort_by_key(|(_, id)| id.0);
        out
    }

    /// Bind a `let`-computed [`Derived`](crate::compute::Derived) value into the
    /// KB. A later [`observed_value`](Self::observed_value) of its name returns
    /// the computed value, and a formula can reference it by name.
    pub fn add_derived(&mut self, derived: crate::compute::Derived) {
        self.derived.push(derived);
    }

    /// Look up a bound derived value by name (most-recently-bound wins, so a
    /// rebinding supersedes — mirroring the latest-observation rule for facts).
    pub fn derived_for(&self, name: &str) -> Option<&crate::compute::Derived> {
        self.derived.iter().rev().find(|d| d.name == name)
    }

    /// All `let`-bound derived values, in binding order. Read-only view so a
    /// consumer (e.g. the CLI's JSON renderer) can surface every computed
    /// quantity together with the [`Dimension`](crate::dimension::Dimension)
    /// the engine inferred for it — the audit channel for dimensional
    /// analysis. A rebinding leaves both entries here (latest wins for
    /// [`derived_for`](Self::derived_for) / [`observed_value`](Self::observed_value));
    /// the renderer keeps the most-recent per name to mirror that rule.
    pub fn derived_bindings(&self) -> &[crate::compute::Derived] {
        &self.derived
    }

    /// True iff at least one contribution (single or joint) names
    /// `conclusion`. This is the discriminator that `AutoDetect` uses
    /// to route to LR aggregation rather than SLD / WMC. Uncertainty
    /// markers alone do not promote a query to LR-aggregation —
    /// they're meaningful only relative to contribution clauses,
    /// because they describe domain gaps inside an LR-shape query.
    pub fn participates_in_lr_aggregation(&self, conclusion: &Term) -> bool {
        self.contributions
            .iter()
            .any(|c| &c.conclusion == conclusion)
            || self
                .joint_contributions
                .iter()
                .any(|c| &c.conclusion == conclusion)
            || self
                .predicate_contributions
                .iter()
                .any(|c| &c.conclusion == conclusion)
    }

    /// Insert an [`UncertaintyMarker`], assigning a fresh
    /// [`UncertaintyMarkerId`].
    pub fn add_uncertainty_marker(&mut self, mut marker: UncertaintyMarker) -> UncertaintyMarkerId {
        let id = UncertaintyMarkerId(self.next_uncertainty_marker_id);
        self.next_uncertainty_marker_id += 1;
        marker.id = id;
        self.uncertainty_markers.push(marker);
        id
    }

    /// Iterate the uncertainty markers attached to `conclusion`.
    pub fn uncertainty_markers_for(&self, conclusion: &Term) -> Vec<&UncertaintyMarker> {
        self.uncertainty_markers
            .iter()
            .filter(|m| &m.conclusion == conclusion)
            .collect()
    }

    /// LP19e "observation gate." If `evidence_term` is asserted in the KB as a
    /// `Certain` Fact, return that direct observation. Otherwise fall back to
    /// SLD proof enumeration: if the evidence is derivable by rules (or by
    /// probabilistic facts), return the strongest proof and its probability
    /// product as an attenuation factor for the LR step.
    pub fn observed_evidence(&self, evidence_term: &Term) -> Option<ObservedEvidence> {
        let matched: Vec<FactId> = self
            .facts_for(evidence_term)
            .iter()
            .filter(|f| f.probability == Probability::Certain && &f.term == evidence_term)
            .map(|f| f.id)
            .collect();
        if !matched.is_empty() {
            return Some(ObservedEvidence {
                fact_ids: matched,
                rule_ids: Vec::new(),
                proof: None,
                confidence: 1.0,
            });
        }

        let dag = enumerate_all(evidence_term, self);
        dag.proofs
            .into_iter()
            .map(|proof| {
                let confidence = proof_confidence(&proof, self);
                (proof, confidence)
            })
            .filter(|(_, confidence)| *confidence > 0.0)
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(proof, confidence)| ObservedEvidence {
                fact_ids: proof.via_facts.clone(),
                rule_ids: proof.via_rules.clone(),
                proof: Some(Box::new(proof)),
                confidence,
            })
    }
}

fn proof_confidence(proof: &Proof, kb: &KnowledgeBase) -> f64 {
    let fact_confidence = proof
        .via_facts
        .iter()
        .map(|id| {
            kb.find_fact_by_id(*id)
                .map(|fact| probability_confidence(fact.probability))
                .unwrap_or(0.0)
        })
        .product::<f64>();
    let rule_confidence = proof
        .via_rules
        .iter()
        .map(|id| {
            kb.find_rule_by_id(*id)
                .map(|rule| probability_confidence(rule.probability))
                .unwrap_or(0.0)
        })
        .product::<f64>();
    fact_confidence * rule_confidence
}

fn probability_confidence(probability: Probability) -> f64 {
    match probability {
        Probability::Certain => 1.0,
        Probability::Value(p) if p.is_finite() => p.clamp(0.0, 1.0),
        Probability::Value(_) => 0.0,
    }
}

/// Extract the numeric **magnitude** of a typed value term, if it has one.
///
/// The ADJ language expansion (step 2) models a fact's value as either a
/// bare number or a *typed-value wrapper* that carries the magnitude as
/// its leading argument and the unit/currency afterward:
///
/// | surface value           | term shape                              | magnitude |
/// |-------------------------|-----------------------------------------|-----------|
/// | `18000`                 | `Num(18000)`                            | `18000.0` |
/// | `quantity(18000, usd)`  | `Compound{quantity, [Num(18000), usd]}` | `18000.0` |
/// | `money(18000, usd)`     | `Compound{money, [Num(18000), usd]}`    | `18000.0` |
/// | `percentage(40)`        | `Compound{percentage, [Num(40)]}`       | `40.0`    |
/// | `duration(365, days)`   | `Compound{duration, [Num(365), days]}`  | `365.0`   |
/// | `count(3)`              | `Compound{count, [Num(3)]}`             | `3.0`     |
///
/// The rule is uniform — "the leading numeric argument" — so we do not
/// hard-code a closed set of wrapper functors; any compound that puts a
/// number first exposes that number as its magnitude. A predicate
/// (`gross_income >= 14600`) compares against this magnitude while the
/// unit stays attached to the fact for the faithfulness gate. Returns
/// `None` for symbolic terms with no leading number.
pub fn numeric_magnitude(value: &Term) -> Option<f64> {
    match value {
        Term::Num(Number::Int(i)) => Some(*i as f64),
        Term::Num(Number::Float(x)) => Some(*x),
        // An exactly-stored decimal (ADJ-EXACT-NUMBERS NX-2) reads out as its labeled-lossy `f64`
        // magnitude here — the same value the old `Float(f64)` path yielded, since a valued fact's
        // magnitude flows into the inherently-`f64` compute layer. (Exact-rational ingestion of the
        // decimal, with no `f64` hop, is NX-3.)
        Term::Num(Number::Exact(d)) => Some(d.to_f64()),
        // Typed wrapper: the magnitude is the leading numeric argument.
        Term::Compound { args, .. } => match args.first() {
            Some(Term::Num(Number::Int(i))) => Some(*i as f64),
            Some(Term::Num(Number::Float(x))) => Some(*x),
            Some(Term::Num(Number::Exact(d))) => Some(d.to_f64()),
            _ => None,
        },
        _ => None,
    }
}

/// Exact counterpart to [`numeric_magnitude`]. `Int` and `Exact(BigDecimal)` values ingest
/// **exactly** — the decimal is `mantissa × 10^(-scale)`, converted to its true `BigRational`
/// with no `f64` hop (NX-3). A `Float` term is the one inexact ingress: it captures only
/// *integer-valued* floats (via [`from_integer_f64`](crate::compute::ExactRational::from_integer_f64))
/// and returns `None` for a fractional binary float, whose intended exact form belongs to the
/// base-10 literal string handled by the language adapter, not to the rounded `f64`.
pub fn numeric_exact_magnitude(value: &Term) -> Option<crate::compute::ExactRational> {
    match value {
        Term::Num(Number::Int(i)) => Some(crate::compute::ExactRational::from_i128(*i as i128)),
        Term::Num(Number::Float(x)) => crate::compute::ExactRational::from_integer_f64(*x),
        // NX-3 ingests an exactly-stored decimal at *full precision* — `BigDecimal` is
        // `mantissa × 10^(-scale)`, an exact ratio, so `to_rational()` hands the compute layer the
        // true value with **no `f64` hop**. A stored 39-digit pi therefore stays exact through
        // arithmetic (`pi * 2`), instead of collapsing to the ~16-digit nearest float the NX-2
        // stopgap produced. `Int` above is already exact; `Float` remains the one inexact ingress.
        Term::Num(Number::Exact(d)) => {
            Some(crate::compute::ExactRational::from_ratio(d.to_rational()))
        }
        Term::Compound { args, .. } => match args.first() {
            Some(Term::Num(Number::Int(i))) => {
                Some(crate::compute::ExactRational::from_i128(*i as i128))
            }
            Some(Term::Num(Number::Float(x))) => {
                crate::compute::ExactRational::from_integer_f64(*x)
            }
            Some(Term::Num(Number::Exact(d))) => {
                Some(crate::compute::ExactRational::from_ratio(d.to_rational()))
            }
            _ => None,
        },
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Search modes (only FindFirst is implemented in this slice)
// ---------------------------------------------------------------------------

/// Per LP19 and LP19e, four search modes are defined.
/// - [`FindFirst`](Self::FindFirst) stops at the first successful
///   derivation.
/// - [`EnumerateAll`](Self::EnumerateAll) traverses every branch
///   and returns the complete proof DAG together with the
///   weighted-model-counting posterior.
/// - [`LRAggregate`](Self::LRAggregate) computes a likelihood-ratio
///   Bayesian posterior over a `prior + Σ contributes` rulebook.
/// - [`AutoDetect`](Self::AutoDetect) chooses among the three above
///   per query, using `kb.participates_in_lr_aggregation(query)` to
///   pick LR aggregation, then `kb.is_all_certain()` to pick between
///   FindFirst and EnumerateAll. See the LP19e §"AutoDetect: extended
///   routing" decision tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    FindFirst,
    EnumerateAll,
    LRAggregate,
    AutoDetect,
}

/// What a search call returns.
///
/// - [`FindFirstResult`](Self::FindFirstResult) — at most one
///   substitution, no DAG. Returned by `FindFirst`.
/// - [`EnumerateAllResult`](Self::EnumerateAllResult) — full proof
///   DAG plus the WMC posterior. Returned by `EnumerateAll`.
/// - [`LRAggregateResult`](Self::LRAggregateResult) — single-proof
///   DAG whose steps enumerate the prior and every active LR
///   contribution in evaluation order, with the posterior and its
///   log-odds. Returned by `LRAggregate`. Carries any
///   [`LrAggregateWarning`]s the algorithm raised.
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
    /// LP19e likelihood-ratio aggregation. The DAG has exactly one
    /// `Proof` whose steps enumerate the prior and every active
    /// contribution; `posterior_logit` is the final running log-odds
    /// and `posterior` is its sigmoid.
    LRAggregateResult {
        dag: ProofDAG,
        posterior: f64,
        posterior_logit: f64,
        warnings: Vec<LrAggregateWarning>,
        /// Active uncertainty reports — one per
        /// [`UncertaintyMarker`] on the query whose domain is
        /// entirely unobserved. Empty in the common case.
        uncertainties: Vec<UncertaintyReport>,
    },
}

/// Run a query against the KB under the chosen search mode. When mode
/// is `AutoDetect`, the engine inspects `kb.is_all_certain()` and picks
/// `FindFirst` if every clause is `Certain`, otherwise `EnumerateAll`
/// — this is the LP19 short-circuit theorem made explicit.
pub fn search(query: &Term, kb: &KnowledgeBase, mode: SearchMode) -> SearchResult {
    // LP19e §"AutoDetect: extended routing": LR aggregation takes
    // priority over the WMC short-circuit, because if a conclusion
    // is the target of `contributes` clauses, the user has declared
    // the Bayesian shape and we should honour it even if every
    // other clause in the KB is Certain.
    let effective = match mode {
        SearchMode::FindFirst => SearchMode::FindFirst,
        SearchMode::EnumerateAll => SearchMode::EnumerateAll,
        SearchMode::LRAggregate => SearchMode::LRAggregate,
        SearchMode::AutoDetect => {
            if kb.participates_in_lr_aggregation(query) {
                SearchMode::LRAggregate
            } else if kb.is_all_certain() {
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
        SearchMode::LRAggregate => {
            let result = lr_aggregate(query, kb);
            SearchResult::LRAggregateResult {
                dag: result.dag,
                posterior: result.posterior,
                posterior_logit: result.posterior_logit,
                warnings: result.warnings,
                uncertainties: result.uncertainties,
            }
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
                .or_insert_with(|| LogicVar::fresh(v.display_name.as_deref()))
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
    // A depth-capped search that gives up reports "no proof" to this API's
    // `Option` return. That is the SAFE direction here: `find_first` is a
    // "can you prove it?" question, and answering "I could not" after an
    // exhausted budget is honest. The danger is entirely on the NEGATION side,
    // where "could not prove" is read as "is false" — see `prove_body`, which
    // consumes the `Result` directly and refuses to make that inference.
    find_first_with(query, kb, &Substitution::empty(), 0).unwrap_or(None)
}

fn find_first_with(
    query: &Term,
    kb: &KnowledgeBase,
    subst: &Substitution,
    depth: usize,
) -> Result<Option<Substitution>, ResolutionLimitExceeded> {
    // Same guard, same reason, as the enumeration resolver: without it a
    // self-recursive rule descends until the process aborts on a stack
    // overflow, which is a SIGABRT an embedding process cannot catch.
    if depth >= MAX_SLD_DEPTH {
        return Err(ResolutionLimitExceeded);
    }
    let resolved = subst.walk(query);

    // Try facts first — they have no body, so success is immediate.
    for fact in kb.facts_for(&resolved) {
        let mut renames = HashMap::new();
        let renamed = rename_term(&fact.term, &mut renames);
        if let Some(s) = unify(&resolved, &renamed, subst) {
            return Ok(Some(s));
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
            if prove_body(&renamed_body, kb, &mut s, depth + 1)? {
                return Ok(Some(s));
            }
        }
    }

    Ok(None)
}

/// Prove every literal in `body` under the substitution `s`, threading
/// each successful subgoal's resulting substitution forward to the next.
fn prove_body(
    body: &[BodyLiteral],
    kb: &KnowledgeBase,
    s: &mut Substitution,
    depth: usize,
) -> Result<bool, ResolutionLimitExceeded> {
    for literal in body {
        match literal {
            BodyLiteral::Pos(t) => match find_first_with(t, kb, s, depth)? {
                Some(next) => *s = next,
                None => return Ok(false),
            },
            BodyLiteral::Neg(t) => {
                // `?` IS LOAD-BEARING. If the negated goal's own search hit the
                // depth cap we must propagate, not observe `None`. Reading an
                // exhausted budget as "not provable" would make `not G` succeed
                // because we STOPPED LOOKING — turning a resource limit into a
                // positive claim about the world. Same hazard, and same fix, as
                // the enumeration resolver's negation branch.
                if find_first_with(t, kb, s, depth)?.is_some() {
                    // The negated goal is provable — negation-as-failure fails.
                    return Ok(false);
                }
                // Goal not provable; negation succeeds; substitution unchanged.
            }
        }
    }
    Ok(true)
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
    fn derived_bindings_exposes_every_let_in_binding_order() {
        // The CLI's dimensional-audit channel reads this accessor. It must
        // return every binding (so a UI can list all computed quantities),
        // in binding order, while `derived_for` keeps the latest-wins rule.
        let mut kb = empty_kb();
        assert!(kb.derived_bindings().is_empty());
        let a = compute("a", &ComputeExpr::Lit(2.0), &kb).unwrap();
        let b = compute(
            "b",
            &ComputeExpr::Bin(
                ComputeOp::Add,
                Box::new(ComputeExpr::Lit(1.0)),
                Box::new(ComputeExpr::Lit(4.0)),
            ),
            &kb,
        )
        .unwrap();
        kb.add_derived(a);
        kb.add_derived(b);
        let bindings = kb.derived_bindings();
        assert_eq!(bindings.len(), 2);
        assert_eq!(bindings[0].name, "a");
        assert_eq!(bindings[1].name, "b");
        assert_eq!(bindings[1].value, 5.0);
        // A rebinding appends; the table keeps both, latest wins for lookup.
        let a2 = compute("a", &ComputeExpr::Lit(9.0), &kb).unwrap();
        kb.add_derived(a2);
        assert_eq!(kb.derived_bindings().len(), 3);
        assert_eq!(kb.derived_for("a").unwrap().value, 9.0);
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
            compound(
                "grandfather",
                vec![Term::Var(xx.clone()), Term::Var(zz.clone())],
            ),
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
        let query = compound("grandfather", vec![atom("grandpa"), Term::Var(who.clone())]);
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
        kb.add_fact(Fact::certain(compound("q", vec![atom("a"), atom("a")])));
        kb.add_fact(Fact::certain(compound("q", vec![atom("b"), atom("c")])));

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

    // -----------------------------------------------------------------------
    // ADJ73 PR-B-2 — GROUNDED context-precedence edges.
    //
    // A `relate outranks_context(higher, lower)` clause lowers to an ordinary
    // ground Fact. These tests prove that such a fact PARTICIPATES in the
    // context order exactly like an explicit `add_context_outranks` edge — so
    // the *reason* one context outranks another (the Supremacy Clause, a
    // circuit-precedence rule, a guideline year) can ride on the edge as
    // byte-provenance instead of being asserted bare in host code.
    // -----------------------------------------------------------------------

    /// Helper: a ground `outranks_context(higher, lower)` edge fact.
    fn outranks_context_fact(higher: &str, lower: &str) -> Fact {
        Fact::certain(compound(
            "outranks_context",
            vec![atom(higher), atom(lower)],
        ))
    }

    #[test]
    fn grounded_outranks_context_fact_is_a_context_edge() {
        // No explicit add_context_outranks — the ONLY edge is the grounded fact.
        let mut kb = empty_kb();
        kb.add_fact(outranks_context_fact("federal", "state"));
        assert!(
            kb.context_outranks("federal", "state"),
            "a grounded outranks_context fact should drive the context order"
        );
        // Direction matters: the reverse does not hold.
        assert!(!kb.context_outranks("state", "federal"));
        // A context never outranks itself.
        assert!(!kb.context_outranks("federal", "federal"));
    }

    #[test]
    fn grounded_edges_compose_transitively_with_explicit_edges() {
        // Mix the two sources: explicit federal > circuit, grounded circuit > district.
        // The transitive reach federal → district must hold across both kinds of edge.
        let mut kb = empty_kb();
        kb.add_context_outranks("federal", "circuit");
        kb.add_fact(outranks_context_fact("circuit", "district"));
        assert!(kb.context_outranks("federal", "circuit"));
        assert!(kb.context_outranks("circuit", "district"));
        assert!(
            kb.context_outranks("federal", "district"),
            "transitive reach must span explicit + grounded edges"
        );
    }

    #[test]
    fn cycle_detection_spans_explicit_and_grounded_edges() {
        // A cycle formed ACROSS the two sources (explicit a > b, grounded b > a)
        // must still be caught — else the loader would accept a contradictory order.
        let mut kb = empty_kb();
        kb.add_context_outranks("a", "b");
        kb.add_fact(outranks_context_fact("b", "a"));
        assert!(kb.context_order_has_cycle());
        // The resolver stays safe regardless: a mutual outrank is reported both ways
        // (the caller's defeats() then yields a peer/conflict, never a wrong pick).
        assert!(kb.context_outranks("a", "b") && kb.context_outranks("b", "a"));
    }

    #[test]
    fn grounded_edge_carries_retrievable_provenance() {
        // The whole point of "grounded context": the fact that establishes the edge
        // is queryable and its citation is retrievable — the edge explains itself.
        let mut kb = empty_kb();
        let prov = Provenance::new(
            "U.S. Const. art. VI, cl. 2 (Supremacy Clause)",
            Some("cl. 2".to_string()),
            TrustTier::Authoritative,
        );
        let id =
            kb.add_fact(outranks_context_fact("federal", "state").with_provenance(prov.clone()));
        // The edge is live...
        assert!(kb.context_outranks("federal", "state"));
        // ...AND its source is recoverable for the audit trail.
        let f = kb
            .fact(id)
            .expect("the grounded edge fact is stored and queryable");
        assert_eq!(f.provenance.source, prov.source);
        assert_eq!(f.provenance.trust_tier, TrustTier::Authoritative);
    }

    #[test]
    fn non_atom_outranks_context_fact_is_not_an_edge() {
        // A variable/compound arg is not a ground edge — it stays an ordinary fact
        // and does not silently inject a precedence relation.
        let mut kb = empty_kb();
        kb.add_fact(Fact::certain(compound(
            "outranks_context",
            vec![atom("federal"), Term::Var(var("X"))],
        )));
        assert!(!kb.context_outranks("federal", "state"));
        assert!(!kb.context_order_has_cycle());
    }

    // -----------------------------------------------------------------------
    // ADJ73 PR-B-4 — DERIVED context-precedence edges (grounded meta-rules).
    //
    // The precedence ORDER itself can be derived: a meta-rule
    //   outranks_context(H, L) :- reverses(H, L).
    // (the appeal-status canon) turns a primitive grounded fact `reverses(a, b)`
    // into a precedence edge a > b. These tests prove a rule-derived edge drives
    // `context_outranks` exactly like an asserted one — the recursive structure
    // ADJ73 §7 calls for (an edge that can be derived is derived, not duplicated).
    // -----------------------------------------------------------------------

    /// Helper: the appeal-status meta-rule `outranks_context(H, L) :- reverses(H, L)`.
    fn reverses_metarule() -> Rule {
        let h = var("H");
        let l = var("L");
        Rule::certain(
            compound(
                "outranks_context",
                vec![Term::Var(h.clone()), Term::Var(l.clone())],
            ),
            vec![BodyLiteral::Pos(compound(
                "reverses",
                vec![Term::Var(h), Term::Var(l)],
            ))],
        )
    }

    #[test]
    fn metarule_derived_edge_drives_context_outranks() {
        // No asserted outranks_context edge — only a primitive `reverses` fact + the meta-rule.
        let mut kb = empty_kb();
        kb.add_rule(reverses_metarule());
        kb.add_fact(Fact::certain(compound(
            "reverses",
            vec![atom("scotus_2023"), atom("ninth_circuit_2019")],
        )));
        assert!(
            kb.context_outranks("scotus_2023", "ninth_circuit_2019"),
            "the meta-rule should DERIVE the precedence edge from the reverses fact"
        );
        assert!(!kb.context_outranks("ninth_circuit_2019", "scotus_2023"));
        assert!(!kb.context_order_has_cycle());
    }

    #[test]
    fn derived_and_explicit_edges_compose_transitively() {
        // explicit federal > scotus_2023; derived scotus_2023 > ninth_circuit_2019 (via reverses).
        // Transitive reach federal → ninth_circuit_2019 must hold across both kinds.
        let mut kb = empty_kb();
        kb.add_context_outranks("federal", "scotus_2023");
        kb.add_rule(reverses_metarule());
        kb.add_fact(Fact::certain(compound(
            "reverses",
            vec![atom("scotus_2023"), atom("ninth_circuit_2019")],
        )));
        assert!(kb.context_outranks("federal", "ninth_circuit_2019"));
    }

    #[test]
    fn derived_edges_are_cycle_checked() {
        // Two reverses facts forming a cycle (a reverses b, b reverses a) → detectable, never a
        // wrong silent pick. (Degenerate, but the resolver must stay safe on contradictory input.)
        let mut kb = empty_kb();
        kb.add_rule(reverses_metarule());
        kb.add_fact(Fact::certain(compound(
            "reverses",
            vec![atom("ruling_a"), atom("ruling_b")],
        )));
        kb.add_fact(Fact::certain(compound(
            "reverses",
            vec![atom("ruling_b"), atom("ruling_a")],
        )));
        assert!(kb.context_order_has_cycle());
    }

    #[test]
    fn no_metarule_keeps_the_ground_fact_fast_path() {
        // With no outranks_context RULES, only ground facts are edges (back-compat / cheap path).
        // A `reverses` fact alone (no meta-rule) injects NO precedence.
        let mut kb = empty_kb();
        kb.add_fact(Fact::certain(compound(
            "reverses",
            vec![atom("a"), atom("b")],
        )));
        assert!(!kb.context_outranks("a", "b"));
        // A ground outranks_context fact still works without any rule.
        kb.add_fact(outranks_context_fact("a", "b"));
        assert!(kb.context_outranks("a", "b"));
    }
}
