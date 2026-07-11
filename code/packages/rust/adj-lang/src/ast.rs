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
    /// A logic VARIABLE — the `$Enzyme` surface form (MYCIN-2026 REL-2). Appears
    /// only as a *compound argument* in a binding query goal
    /// (`? deficient_in(tay_sachs, $Enzyme)`); the lowerer maps it to a
    /// `logic_core::Term::Var`, and the engine's unification binds it to whatever
    /// the matching ground edge holds. The carried name is the bare identifier
    /// (without the `$`); equal names within one goal lower to the SAME variable.
    Var(String),
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
/// `from answer == 3 / 10 to opt_a`  →  [`Evidence::Predicate`]
///
/// The predicate form is the surface syntax for a deterministic rule:
/// it lowers to a predicate-gated contribution whose likelihood ratio
/// is large enough to saturate. The comparison itself runs on the CPU
/// at decision time — the model that authored the rulebook never
/// evaluated it.
#[derive(Debug, Clone, PartialEq)]
pub enum Evidence {
    Term(Term),
    Predicate {
        slot: String,
        op: CmpOp,
        rhs: ExprAst,
    },
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

/// A binary arithmetic operator in a `let` formula.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithOp {
    Add,
    Sub,
    Mul,
    Div,
    /// Exponentiation, `base ^ exponent`. Produced by the `latex "…"` adapter
    /// for `x^n` (lowered to `logic_engine::ComputeOp::Pow`); the surface
    /// arithmetic grammar does not yet spell `^` directly.
    Pow,
    /// Modulo, `a mod b` — the remainder carrying the sign of the dividend
    /// (`7 mod 3 = 1`, `−7 mod 3 = −1`). Produced by the `latex "…"` adapter for
    /// `a \bmod b` / `a \pmod{b}` (lowered to `logic_engine::ComputeOp::Mod`); the
    /// surface arithmetic grammar does not spell `mod` directly. Like the engine op
    /// it combines dimensionally like addition (operands share a dimension, the
    /// remainder carries it) and rejects a zero divisor.
    Mod,
}

/// A named **transcendental** function in a `let` formula — the curated
/// single-argument set the `latex "…"` surface understands. Each maps to a
/// native transcendental `logic_engine::ComputeOp` (`Scalar → Scalar`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedFn {
    Sin,
    Cos,
    Tan,
    /// Natural logarithm (`\ln`).
    Ln,
    /// Base-10 logarithm (`\log`).
    Log,
    /// Exponential, `e^x` (`\exp`).
    Exp,
    /// Inverse sine (`\arcsin`).
    Asin,
    /// Inverse cosine (`\arccos`).
    Acos,
    /// Inverse tangent (`\arctan`).
    Atan,
    /// Hyperbolic sine (`\sinh`).
    Sinh,
    /// Hyperbolic cosine (`\cosh`).
    Cosh,
    /// Hyperbolic tangent (`\tanh`).
    Tanh,
    /// Cotangent, cos/sin (`\cot`).
    Cot,
    /// Secant, 1/cos (`\sec`).
    Sec,
    /// Cosecant, 1/sin (`\csc`).
    Csc,
}

/// A **binary** named function in a `let` formula — the two-argument set the
/// `latex "…"` surface understands. Each maps to a native binary
/// `logic_engine::ComputeOp` carried in a `ComputeExpr::Bin`. Kept distinct from
/// the single-argument [`NamedFn`] (so the arity is honest) and from the
/// slot-reducing [`AggOp`] `Min`/`Max` (which fold *every* observation of one
/// slot, not two sub-expressions).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinFn {
    /// `\min(a, b)` — the smaller of two quantities.
    Min,
    /// `\max(a, b)` — the larger of two quantities.
    Max,
    /// `\gcd(a, b)` — greatest common divisor (integer operands).
    Gcd,
    /// `\lcm(a, b)` — least common multiple (integer operands).
    Lcm,
}

/// An aggregation operator in a `let` formula — reduces every
/// observation of a slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggOp {
    Sum,
    Count,
    Min,
    Max,
    Avg,
}

/// The formula of a `let <name> = <expr>` binding. Mirrors
/// `logic_engine::ComputeExpr`; kept as a separate surface type so the
/// AST has no engine dependency (the lowerer converts it).
#[derive(Debug, Clone, PartialEq)]
pub enum ExprAst {
    /// A reference to a slot (observed fact or a previously-bound `let`).
    Ref(String),
    /// A numeric literal written into the formula.
    Lit(f64),
    /// A binary arithmetic operation.
    Bin(ArithOp, Box<ExprAst>, Box<ExprAst>),
    /// An absolute value, `|x|` — a unary arithmetic form. Lowers to the native
    /// `ComputeOp::Abs` (dimension-preserving). Produced by the `latex "…"`
    /// adapter from a `|…|`/`\left|…\right|` fence.
    Abs(Box<ExprAst>),
    /// A floor, `⌊x⌋` — the greatest integer ≤ x. Lowers to the native
    /// `ComputeOp::Floor` (dimension-preserving). Produced by the `latex "…"`
    /// adapter from a `\left\lfloor…\right\rfloor` fence.
    Floor(Box<ExprAst>),
    /// A ceiling, `⌈x⌉` — the least integer ≥ x. Lowers to the native
    /// `ComputeOp::Ceil` (dimension-preserving). Produced by the `latex "…"`
    /// adapter from a `\left\lceil…\right\rceil` fence.
    Ceil(Box<ExprAst>),
    /// A round-to-nearest, `⌊x⌉` (ties away from zero). Lowers to the native
    /// `ComputeOp::Round` (dimension-preserving). Produced by the `latex "…"`
    /// adapter from the nearest-integer fence `\left\lfloor…\right\rceil`
    /// (floor-left, ceil-right).
    Round(Box<ExprAst>),
    /// A truncation toward zero, `trunc(x)` — drop the fractional part, keeping the
    /// sign (`trunc(3.7) = 3`, `trunc(−3.7) = −3`; contrast `Floor`, which rounds
    /// toward −∞). Lowers to the native `ComputeOp::Trunc` (dimension-preserving).
    /// Produced by the `latex "…"` adapter from a `\operatorname{trunc}(x)` — the
    /// operator-name juxtaposition (`Text("trunc")` implicitly multiplied by its
    /// parenthesised argument).
    Trunc(Box<ExprAst>),
    /// The **sign** function `sgn(x)` — `−1`/`0`/`+1` for a negative/zero/positive
    /// operand. Lowers to the native `ComputeOp::Sign`, which — unlike the
    /// dimension-preserving rounding ops — collapses the result to a dimensionless
    /// `Scalar` (a sign is a pure number) while accepting a dimensioned operand.
    /// Produced by the `latex "…"` adapter from a `\operatorname{sgn}(x)` — the
    /// operator-name juxtaposition (`Text("sgn")` implicitly multiplied by its
    /// parenthesised argument), exactly like `\operatorname{trunc}`.
    Sign(Box<ExprAst>),
    /// A named **transcendental** function applied to one argument
    /// (`\sin(x)`, `\ln(x)`, `\exp(x)`, …). Lowers to the matching native
    /// transcendental `ComputeOp` (`Scalar → Scalar`, exact dropped). Produced by
    /// the `latex "…"` adapter from a `MathExpr::Call`.
    Call(NamedFn, Box<ExprAst>),
    /// A **binary** named function applied to two arguments (`\min(a, b)`,
    /// `\max(a, b)`). Lowers to the matching native binary `ComputeOp`
    /// (`ComputeOp::Min2`/`Max2`) carried in a `ComputeExpr::Bin`. Produced by the
    /// `latex "…"` adapter from a `MathExpr::Call` whose argument is a
    /// two-element `Sequence`.
    Call2(BinFn, Box<ExprAst>, Box<ExprAst>),
    /// An aggregation over every observation of a slot.
    Agg(AggOp, String),
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
    /// `relate <rel>(<args>)` — assert a ground RELATIONAL EDGE (MYCIN-2026
    /// REL-2). The `edge` term's functor is the relation and its arguments are
    /// the entities (e.g. `deficient_in(tay_sachs, hexosaminidase_a)`). Lowers
    /// to a `logic_engine::Fact` carrying the annotations as its provenance, so a
    /// binding query answer can be returned with the citing edge as its proof.
    Relate {
        edge: Term,
        annotations: Vec<Annotation>,
    },
    /// `rule { head: <term> when: <lit>, <lit> … }` — a DERIVATION RULE (a Horn
    /// clause / Datalog rule). Where `Relate` asserts a GROUND edge, a `Rule` lets
    /// the engine DERIVE `head` whenever every body literal holds under the current
    /// substitution (variables bind across head and body). A literal prefixed with
    /// `not` is negation-as-failure. Lowers to a `logic_engine::Rule { head, body }`,
    /// so `? head($X)` enumerates every derivable answer via the same SLD machinery
    /// `relate` facts resolve through. This is the primitive that lets a `rulebook`
    /// express conditional domain knowledge (contraindications, step-therapy, …) —
    /// authored once, grounded into the CAS, derived by the engine from per-case facts.
    Rule {
        head: Term,
        body: Vec<RuleLiteral>,
        annotations: Vec<Annotation>,
        /// ADJ73 defeasible precedence (PR-C): the optional `priority: <tier>` annotation
        /// (`default` | `specific` | `authoritative` | `mandatory`). `None` lowers to
        /// `Priority::Default`. Among *conflicting* derivations of a functional predicate, a
        /// higher tier defeats a lower one (`logic_engine::govern::enumerate_governing`).
        priority: Option<String>,
        /// ADJ73 PR-B: the optional `context: <name>` annotation — the CONTEXT this rule is
        /// grounded in (jurisdiction / guideline edition / specialty). `None` = context-free.
        /// Lowers to `logic_engine::Rule::with_context`; a rule in a context that outranks
        /// another's (per [`Statement::ContextOrder`]) defeats it before the tier is consulted.
        context: Option<String>,
    },
    /// `functional <pred>(<arg>, …)` — declare a predicate FUNCTIONAL on its last argument
    /// (ADJ73 PR-C): at most one value may hold per key (the preceding args). The argument
    /// names are placeholders for readability; only the functor + arity are used. Lowers to
    /// `KnowledgeBase::declare_functional(functor, arity)`. Two derivations sharing the key but
    /// differing on the last arg then *conflict*, and precedence picks the governing one.
    Functional { functor: String, arity: usize },
    /// `context_order { higher > lower, … }` — ADJ73 PR-B: grounded CONTEXT precedence edges.
    /// Each `(higher, lower)` lowers to `KnowledgeBase::add_context_outranks`, so a rule in
    /// `higher` defeats a conflicting one in `lower` (lex superior).
    ContextOrder { edges: Vec<(String, String)> },
    /// `? <conclusion>` — query the engine. With a ground hypothesis term this
    /// returns the posterior; with a relational goal containing a `$variable`
    /// (`? deficient_in(tay_sachs, $E)`) it returns the binding(s) — fact recall
    /// as the single-hop special case of the differential.
    Query { conclusion: Term },
    /// `let <name> = <expr>` — bind a **computed** value (ADJ expansion
    /// step 3). The model writes only the formula; the engine evaluates
    /// `expr` on the CPU into a derivation tree and binds it to `name`,
    /// after which a predicate can fire over it like an observed slot.
    Let { name: String, expr: ExprAst },
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
    /// `symbol <name> : <sort>` — declare an **unknown** the engine will
    /// solve for (ADJ constraints, track B). `sort` is a dimensional sort
    /// term (`scalar`, `money(usd)`, …).
    Symbol { name: String, sort: Term },
    /// `constrain <lhs> <relop> <rhs>` — assert an (in)equality the solver
    /// must satisfy. Operands reuse the `let` arithmetic [`ExprAst`], so a
    /// constraint may mention observed slots, earlier `let`s, and symbols.
    Constrain {
        lhs: ExprAst,
        op: RelOp,
        rhs: ExprAst,
    },
    /// `solve for { a, b, … }` — drive the solver to find values for the
    /// named unknowns satisfying the accumulated constraints.
    SolveFor { names: Vec<String> },
    /// `check` — ask whether the accumulated constraint set is satisfiable
    /// (feasibility / contradiction).
    Check,
    /// `minimize <expr>` / `maximize <expr>` — a linear-programming objective
    /// over the declared symbols (ADJ constraints track C2). The solver finds
    /// the optimal value subject to the accumulated `constrain` half-planes.
    Optimize { dir: OptDir, objective: ExprAst },
    /// `define <name> : hypothesis | finding values [v…] [surface "…"]` — a
    /// dictionary entry (MYCIN-2026). Registers a finding/hypothesis term in the
    /// controlled vocabulary; valid bare or inside a `dictionary` block.
    Define(Define),
    /// `dictionary <name> { define … }` — a named controlled vocabulary, a
    /// first-class language construct a rulebook `use`s (M2).
    Dictionary { name: String, defines: Vec<Define> },
    /// `rulebook <name> { … }` — a named, reusable block of clauses (MYCIN-2026
    /// M2). The clauses lower into the KB exactly as if written at top level;
    /// the name lets a rulebook be written once and `import`ed (M3). An inner
    /// `use` binds the dictionary its clauses are vocabulary-checked against.
    Rulebook {
        name: String,
        statements: Vec<Statement>,
    },
    /// `formulabook <name> { use <dict>… formula… }` — a named, importable
    /// collection of reusable, provenanced, PARAMETERIZED formulas
    /// (ADJ-FORMULA-LIBRARIES rung-0). A sibling of [`Statement::Rulebook`]: where
    /// a rulebook groups belief clauses, a formulabook groups `formula`
    /// definitions. `uses` records the dictionaries a `use <dict>` brought into
    /// scope (documentation of the vocabulary the formulas are typed against);
    /// `formulas` are the definitions themselves. The formulabook adds nothing to
    /// the KB directly — its formulas are registered so a later `? name(args)`
    /// can APPLY them (see [`FormulaDef`]).
    Formulabook {
        name: String,
        uses: Vec<String>,
        formulas: Vec<FormulaDef>,
    },
    /// `use <dictionary>` — bind a `dictionary` (by name) as the controlled
    /// vocabulary the enclosing scope's clauses are checked against (MYCIN-2026
    /// M2). Legal at top level or inside a `rulebook`.
    Use(String),
    /// `import "<relative path>"` — splice another `.adj` file's declarations
    /// into this program (MYCIN-2026 M3). The literal string is carried verbatim;
    /// resolution (relative path, idempotency, cycle + bound checks) happens in
    /// [`crate::resolve`] before lowering. A program that still contains an
    /// `Import` at lowering time was compiled without the resolver — a
    /// [`crate::LowerError::UnresolvedImport`].
    Import(String),
}

/// One literal in a [`Statement::Rule`] body: a subgoal `term`, optionally
/// negated. `negated` true is negation-as-failure (`not <term>`) — the term must
/// NOT be derivable under the current substitution.
#[derive(Debug, Clone, PartialEq)]
pub struct RuleLiteral {
    pub negated: bool,
    pub term: Term,
}

/// A reusable, provenanced, PARAMETERIZED formula — the rung-0 substrate of
/// ADJ-FORMULA-LIBRARIES. A `formula bmi(body_mass, height) = body_mass /
/// (height * height)` is, semantically, a **named, importable `let`**: `body` is
/// the same [`ExprAst`] a `let` binds, and the leaves that name a declared
/// parameter are FORMAL PARAMETERS, bound at apply time rather than resolved to
/// an already-`observe`d fact.
///
/// ## Provenance is not a decoration — it is the claim
///
/// A formula asserts a fact about the world ("BMI is mass ÷ height²"). Like a
/// `relate` edge, it carries the SAME provenance envelope — `source` / `locator`
/// / `trust` — captured here as the shared [`Annotation`] vector (so there is
/// ONE provenance surface and ONE lowering path, `annotations_to_provenance`).
/// The lowerer enforces a non-empty `source` on a shipped formula (the
/// provenance-required lint), and stamps the resolved provenance onto the
/// applied value so the computed answer is auditable back to WHY its formula is
/// trusted.
#[derive(Debug, Clone, PartialEq)]
pub struct FormulaDef {
    /// The formula's name — also the functor a consumer applies (`? bmi(...)`)
    /// and the name of the derived value the application binds.
    pub name: String,
    /// The formal parameters, in declaration order. A body [`ExprAst::Ref`]
    /// (or [`ExprAst::Agg`] slot) naming one of these is a parameter reference;
    /// any other free identifier is a compile error (parameter-scoping).
    pub params: Vec<String>,
    /// The formula body — the EXISTING `let` expression AST, reused verbatim.
    pub body: ExprAst,
    /// The provenance envelope (`source` / `locator` / `trust`, plus any
    /// corroborating `cites`), reusing the shared [`Annotation`] set every
    /// grounded clause carries. Lowered via `annotations_to_provenance`; a
    /// shipped formula must carry a non-empty `source`.
    pub annotations: Vec<Annotation>,
}

/// A single dictionary entry (MYCIN-2026).
#[derive(Debug, Clone, PartialEq)]
pub struct Define {
    /// The canonical term (a finding functor like `csf_glucose`, or a
    /// hypothesis like `bacterial_meningitis`).
    pub name: String,
    pub kind: DefineKind,
    /// Surface forms used to constrain the decomposer — *not* engine-semantic.
    pub surfaces: Vec<String>,
}

/// What a `define` registers: a hypothesis, or a finding with a closed value
/// domain (so "observed normal" is distinguishable from "not yet observed").
#[derive(Debug, Clone, PartialEq)]
pub enum DefineKind {
    Hypothesis,
    Finding {
        values: Vec<String>,
    },
    /// `<name> : entity` — a NODE kind in the relational knowledge graph
    /// (MYCIN-2026 REL-2), e.g. `disease`, `enzyme`. Entities are the arguments
    /// relations connect.
    Entity,
    /// `<name> : relation from <domain> to <range>` — a typed EDGE kind, e.g.
    /// `deficient_in : relation from disease to enzyme`. The domain/range name
    /// the entity kinds the relation connects (typed-graph documentation; strict
    /// argument-type enforcement is a later slice).
    Relation {
        from: String,
        to: String,
    },
}

/// The direction of an `optimize` (LP) objective.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptDir {
    /// `minimize <expr>` — find the smallest feasible objective value.
    Minimize,
    /// `maximize <expr>` — find the largest feasible objective value.
    Maximize,
}

/// A relational operator in a `constrain` clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelOp {
    Ge,
    Le,
    Gt,
    Lt,
    /// Equality — surface `=` or `==`.
    Eq,
    /// Inequality — surface `!=`.
    Ne,
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
    /// `cites "<source>" locator "<locator>"` (ADJ-A9) — appends a
    /// corroborating citation to `Provenance::corroborations`. Repeatable;
    /// each carries a required locator so the span is re-fetchable.
    Cites { source: String, locator: String },
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
