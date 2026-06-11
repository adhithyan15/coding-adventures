//! # AST — what the parser produces and the lowerer consumes.
//!
//! Deliberately small and shape-preserving with the surface syntax.
//! Each statement variant maps 1:1 to an LP19e clause kind plus
//! [`Statement::Observe`] (Fact) and [`Statement::Query`] (run a
//! query at the end). Annotations are gathered into a `Vec` and
//! interpreted at lowering time — that keeps the parser simple
//! (one annotation = one rule) while still letting the lowerer
//! enforce ordering / multiplicity invariants.

/// A Term in the surface language: either an atom or a compound
/// term. The lowerer converts these to `logic_core::Term`s.
///
/// Mirrors a small subset of Prolog term shape — sufficient for
/// medical / legal / financial rulebooks where every term is either
/// a flat label (`acs`) or a single-arg predicate (`pmh(hypertension)`).
/// Multi-arg compounds are supported syntactically but rarely used.
#[derive(Debug, Clone, PartialEq)]
pub enum Term {
    Atom(String),
    /// A numeric literal — appears as a compound argument in a *valued*
    /// fact, e.g. `gross_income(18000)`. Carried as `f64`; the lowerer
    /// converts it to `logic_core::Term::Num`. Valued facts are what
    /// predicate-gated contributions read on the CPU.
    Num(f64),
    Compound {
        functor: String,
        args: Vec<Term>,
    },
}

/// The evidence side of a `contributes` clause — either an ordinary
/// term or a numeric **predicate** over a valued slot.
///
/// `from pmh(hypertension) to acs`  →  [`Evidence::Term`]
/// `from gross_income >= 14600 to required_to_file`  →  [`Evidence::Predicate`]
///
/// The predicate form is the surface syntax for a deterministic rule:
/// it lowers to a predicate-gated contribution whose likelihood ratio
/// is large enough to saturate. The comparison itself runs on the CPU
/// at decision time — the model that authored the rulebook never
/// evaluated it.
#[derive(Debug, Clone, PartialEq)]
pub enum Evidence {
    Term(Term),
    Predicate { slot: String, op: CmpOp, value: f64 },
}

/// A numeric comparison operator in a predicate. Mirrors
/// [`logic_engine::CmpOp`]; kept as a separate surface type so the AST
/// has no engine dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Ge,
    Le,
    Gt,
    Lt,
    Eq,
}

/// One source line in an Adj-Lang program.
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// `prior <probability> for <conclusion>` (+ annotations).
    Prior {
        probability: f64,
        conclusion: Term,
        annotations: Vec<Annotation>,
    },
    /// `contributes <lr> from <evidence> to <conclusion>` (+ annotations).
    /// `evidence` is either a term or a numeric predicate (see [`Evidence`]).
    Contributes {
        lr: f64,
        evidence: Evidence,
        conclusion: Term,
        annotations: Vec<Annotation>,
    },
    /// `interacts <lr> when <evidence> and <evidence> [and ...] for
    /// <conclusion>` (+ annotations).
    Interacts {
        lr: f64,
        evidence_set: Vec<Term>,
        conclusion: Term,
        annotations: Vec<Annotation>,
    },
    /// `observe <term>` — assert a Certain Fact.
    Observe { term: Term },
    /// `? <conclusion>` — query the engine for the posterior.
    Query { conclusion: Term },
    /// `uncertain { <e1>, <e2>, ... } for <conclusion>` — annotate
    /// the conclusion with a domain of candidate evidence terms,
    /// none of which has been observed. The LR aggregator surfaces
    /// a VOI report listing what each value would contribute.
    /// Dissolves ADJ46 awkwardness item A5.
    Uncertain {
        domain: Vec<Term>,
        conclusion: Term,
        annotations: Vec<Annotation>,
    },
}

/// Per-statement annotation. Multiple annotations per statement
/// permitted; the lowerer collapses them onto the clause's
/// [`logic_engine::Provenance`].
#[derive(Debug, Clone, PartialEq)]
pub enum Annotation {
    /// `source "<text>"` — sets `Provenance::source`.
    Source(String),
    /// `locator "<text>"` — sets `Provenance::locator`.
    Locator(String),
    /// `trust <tier>` — sets `Provenance::trust_tier`.
    Trust(TrustTierName),
}

/// Surface name for a trust tier — keywords in the language map
/// 1:1 to [`logic_engine::TrustTier`] variants.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrustTierName {
    Consensus,
    Authoritative,
    Empirical,
    Inferred,
    Unattributed,
}

/// A complete program: a sequence of statements.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Statement>,
}
