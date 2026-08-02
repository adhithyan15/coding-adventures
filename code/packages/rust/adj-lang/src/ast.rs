//! # AST — what the parser produces and the lowerer consumes.
//!
//! Deliberately small and shape-preserving with the surface syntax.
//! Each statement variant maps 1:1 to an LP19e clause kind plus
//! [`Statement::Observe`] (Fact) and [`Statement::Query`] (run a
//! query at the end). Annotations are gathered into a `Vec` and
//! interpreted at lowering time — that keeps the parser simple
//! (one annotation = one rule) while still letting the lowerer
//! enforce ordering / multiplicity invariants.

/// A **numeric literal** as written in the source, kept in the shape that loses no digit
/// (ADJ-EXACT-NUMBERS NX-2). Before NX-2 every number went straight to `f64` at parse time, so a
/// 39-digit π literal came back at ~16 digits. `NumLit` fixes that at the AST level:
///
/// - **`Int(i64)`** — a whole number that fits `i64` (`18000`). It lowers to
///   `logic_core::Number::Int`, keeping the engine's small-integer fast paths and exact-integer
///   ergonomics.
/// - **`Exact(BigDecimal)`** — anything else: a fractional or out-of-`i64` decimal, in scientific
///   notation or not (`2.54`, `6.022e23`, π to 39 places). It lowers to
///   `logic_core::Number::Exact`, so every written digit survives to the stored ground value.
///
/// The lossy `f64` still exists, but only where a value is *asked for* as an `f64` — a compute
/// leaf (`ExprAst::Lit`) or an inherently-approximate backend — via [`NumLit::to_f64_lossy`],
/// never as the silent default at parse time.
#[derive(Debug, Clone, PartialEq)]
pub enum NumLit {
    /// A whole number that fits `i64` — lowers to `logic_core::Number::Int`.
    Int(i64),
    /// An exactly-written decimal (fractional, scientific, or beyond `i64`) — lowers to
    /// `logic_core::Number::Exact`, preserving every digit.
    Exact(bignum_core::BigDecimal),
}

impl NumLit {
    /// The **labeled lossy** `f64` view of this literal — the single sanctioned way to obtain an
    /// `f64` from a `NumLit`. `Int` widens with `as f64`; `Exact` narrows to the nearest `f64` via
    /// [`bignum_core::BigDecimal::to_f64`]. Used only where an `f64` is genuinely required (a
    /// compute leaf or an approximate backend), never at parse time. The name is deliberately
    /// greppable so every lossy boundary is auditable, mirroring `logic_core::Number::to_f64_lossy`.
    pub fn to_f64_lossy(&self) -> f64 {
        match self {
            NumLit::Int(i) => *i as f64,
            NumLit::Exact(d) => d.to_f64(),
        }
    }
}

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
    /// fact, e.g. `gross_income(18000)`. Carried as a [`NumLit`] so no digit is
    /// lost at parse time; the lowerer converts it to `logic_core::Term::Num`
    /// (`Int` → `Number::Int`, `Exact` → `Number::Exact`). Valued facts are what
    /// predicate-gated contributions read on the CPU.
    Num(NumLit),
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
/// `from bmi(body_mass, height) >= 30 to obese`  →  [`Evidence::Predicate`]
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
        /// The left-hand side being compared. Ordinarily a bare slot reference
        /// (`ExprAst::Ref("gross_income")`) — an observed or `let`-derived value.
        /// As of ADJ-RULE-SUBSTRATE RS-1 it may also be a **formula application**
        /// (`ExprAst::Apply("bmi", …)`) so a rulebook can *branch on a formula*:
        /// the lowerer computes the formula into a derived value (composing its
        /// provenance) and gates the contribution on that derived slot. Carried as
        /// an [`ExprAst`] rather than a bare `String` precisely so both shapes fit
        /// one field with one lowering path.
        lhs: ExprAst,
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
    /// A **precision narrowing** — `round_to(x, n)` (NUM-6a, decimal places) or
    /// `round_sig(x, n)` (NUM-6b, significant figures). Unlike [`ExprAst::Round`]
    /// (round to the nearest *integer*), this carries a precision spec, so it lowers
    /// to the distinct `logic_engine::ComputeExpr::Round` node (not a unary
    /// `ComputeOp`), rounding on the exact path under the default half-even mode
    /// (ADJ-NUMERIC-SUBSTRATE §4.1–§4.4). The [`logic_engine::RoundSpec`] is already
    /// validated (a non-negative integer within the precision cap; ≥ 1 for
    /// significant figures). Produced by the **native application** surface
    /// `round_to(x, n)` / `round_sig(x, n)` — recognised as built-ins in
    /// [`ExprAst::Apply`] lowering, which reuses the same comma-list argument grammar
    /// as user formula applications (`quotient(a, b)`), so no new grammar or LaTeX
    /// change is needed.
    RoundTo(Box<ExprAst>, logic_engine::RoundSpec),
    /// A **scientific-notation formatting** — `to_scientific(x [, figures])` (NUM-6c).
    /// A rendering op: it narrows `x` to `figures` significant figures on the exact
    /// path (like [`ExprAst::RoundTo`] with `SigFigures`) and produces the `d.ddde±E`
    /// string alongside the narrowed value, so the audit carries both the exact source
    /// and the rendered form (ADJ-NUMERIC-SUBSTRATE §4.1, §4.3). Lowers to the distinct
    /// `logic_engine::ComputeExpr::ToScientific` node under the default half-even mode.
    /// The `figures` count is validated (`≥ 1`, within the precision cap); when the
    /// surface omits it, the default is resolved at lowering. Produced by the native
    /// application surface `to_scientific(x [, figures])`, recognised as a built-in in
    /// [`ExprAst::Apply`] lowering (same comma-list grammar as `round_to`/`round_sig`).
    ToScientific(Box<ExprAst>, u32),
    /// A **percentage formatting** — `to_percent(x [, places])` (NUM-6c). A rendering op:
    /// it takes `x` as a dimensionless ratio, scales it by 100 and rounds to `places`
    /// decimal places on the exact path, and produces the `d.dd%` string alongside the
    /// narrowed fraction (ADJ-NUMERIC-SUBSTRATE §4.1, §4.3). Lowers to the distinct
    /// `logic_engine::ComputeExpr::ToPercent` node under the default half-even mode. The
    /// `places` count is validated (`≥ 0`, within the precision cap); when the surface
    /// omits it, the default is resolved at lowering. Produced by the native application
    /// surface `to_percent(x [, places])`, recognised as a built-in in [`ExprAst::Apply`]
    /// lowering (same comma-list grammar as `round_to`/`to_scientific`).
    ToPercent(Box<ExprAst>, u32),
    /// A **currency formatting** — `to_currency(x, code [, places])` (NUM-6c). A rendering
    /// op like [`ExprAst::ToPercent`] but carrying a currency **code** string (the first
    /// field) alongside the decimal `places` (the second): it renders the money amount `x`
    /// to `places` base-10-exact decimal places and prefixes the code (ADJ-NUMERIC-SUBSTRATE
    /// §4.1, §4.3). Lowers to the distinct `logic_engine::ComputeExpr::ToCurrency` node
    /// under the default half-even mode. `places ≥ 0` (default resolved at lowering); the
    /// `code` is a bare identifier (`USD`) taken verbatim from the surface. Produced by the
    /// native application surface `to_currency(x, code [, places])`, recognised as a built-in
    /// in [`ExprAst::Apply`] lowering (same comma-list grammar as `round_to`/`to_percent`).
    ToCurrency(Box<ExprAst>, String, u32),
    /// An aggregation over every observation of a slot.
    Agg(AggOp, String),
    /// A **formula application** used as a sub-expression: `name(arg₁, …, argₙ)`
    /// (ADJ-RULE-SUBSTRATE RS-1 — the composition core). This is the node that
    /// makes "a formula IS a rule" concrete at the expression level: wherever the
    /// compute grammar appears — inside a formula body (→ *formula-calls-formula*),
    /// inside a `let`, or on the left of a `contributes … from <app> <op> <thr>`
    /// predicate (→ a rulebook *branches on a formula*) — a named formula may be
    /// applied to argument expressions.
    ///
    /// ## How it resolves (lowering, not a second evaluator)
    ///
    /// An `Apply` is not evaluated directly. At lowering time the lowerer looks
    /// the name up in the SAME formula registry the top-level `? name(args)` query
    /// path uses, binds the callee's parameters to the (already-expanded) argument
    /// expressions, and substitutes them into the callee's body — *recursively*, so
    /// a callee body that itself contains an `Apply` expands too. The fully
    /// expanded, `Apply`-free `ExprAst` then lowers through the existing
    /// `lower_expr`/`compute` path. A recursion-depth guard turns a self- or
    /// mutually-recursive formula into a clean `FormulaRecursionTooDeep` error
    /// rather than a stack overflow (see `expand_applies` in `lower.rs`).
    ///
    /// ## Why a distinct node (not `Call`/`Agg`)
    ///
    /// [`ExprAst::Call`]/[`ExprAst::Call2`] are the *built-in* transcendental /
    /// binary functions the `latex "…"` surface understands (a closed set that maps
    /// to native engine ops); [`ExprAst::Agg`] folds every observation of ONE slot.
    /// An `Apply` is different in kind: it names a **user-defined library formula**,
    /// resolved against imported `formulabook`s. Keeping it a separate node makes an
    /// unknown formula (`FormulaUnknown`) and an arity mismatch (`FormulaArity`)
    /// distinguishable from an ordinary aggregation or built-in call, and leaves
    /// `sum(slot)` aggregation untouched (the aggregation keywords are matched
    /// *before* an application in the grammar).
    Apply(String, Vec<ExprAst>),
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
    /// `observe <term>` (+ annotations) — assert a Certain, byte-groundable Fact.
    Observe {
        term: Term,
        annotations: Vec<Annotation>,
    },
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
    /// `? lookup <table> <key_col> = <n> mode <mode> give <value_col>` — a
    /// RANGE / INTERPOLATED lookup over a [`Statement::Table`] (ADJ-TABLES RS-5c/RS-5d).
    /// Unlike [`Query`] (exact SLD unification on the key), the `mode` selects a
    /// numeric tactic over the table's breakpoints:
    /// - `range` reads the table as a **step function**: it selects the breakpoint
    ///   row whose `key_col` is the greatest key `<= key_value` and returns that
    ///   row's `value_col` verbatim — tax brackets, dose bands, reference-range
    ///   classification. A query below the smallest key abstains ("below the table's
    ///   domain").
    /// - `interpolated` reads it as a **piecewise-linear function**: it finds the two
    ///   bracketing rows `k0 <= key_value <= k1` and returns the exact linear blend
    ///   `v0 + (v1−v0)·(key_value−k0)/(k1−k0)` — nomograms, growth charts, calibration
    ///   curves. Both bracketing rows' citations ride along. A query outside `[min,
    ///   max]` abstains (below- or above-domain); it never extrapolates. The value
    ///   column must be numeric ([`crate::LowerError::LookupNonNumericValueColumn`]).
    ///
    /// An unrecognized mode is [`crate::LowerError::LookupUnknownMode`].
    RangeLookup {
        /// The `table` to read as a step / piecewise-linear function (must be a
        /// declared table: [`crate::LowerError::LookupUnknownTable`]).
        table: String,
        /// The numeric key column the query value is compared against (must be one
        /// of the table's columns and hold only numbers:
        /// [`crate::LowerError::LookupUnknownColumn`] /
        /// [`crate::LowerError::LookupNonNumericKeyColumn`]).
        key_col: String,
        /// The concrete query value to classify. Carried as a [`NumLit`] (sign
        /// already folded in) so no digit is lost before the exact comparison.
        key_value: NumLit,
        /// The lookup tactic named at the call site (`range` or `interpolated`).
        mode: String,
        /// The column whose cell is returned for the selected breakpoint row (must
        /// be one of the table's columns: [`crate::LowerError::LookupUnknownColumn`]).
        value_col: String,
    },
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
    /// `table <name> { columns … row(…)… }` — a first-class, importable,
    /// provenanced tabular relation (ADJ-TABLES RS-5). A sibling of
    /// [`Statement::Rulebook`]/[`Statement::Formulabook`]: real reference
    /// knowledge (unit conversions, reference ranges, dose charts, tax brackets)
    /// is tabular, and each `row (v1, …, vn)` lowers to a ground relation
    /// `name(v1, …, vn)` carrying the table's provenance — byte-identical to how
    /// a `relate` edge lowers — so EXACT lookup is the existing SLD binding query
    /// with no new engine machinery. `uses` records `use <dict>` for vocabulary
    /// checking; `columns` fixes the row arity (a row of a different length is a
    /// [`crate::LowerError::TableArity`]); `annotations` is the shared provenance
    /// envelope (a shipped table must be sourced).
    Table {
        name: String,
        uses: Vec<String>,
        columns: Vec<String>,
        rows: Vec<TableRow>,
        annotations: Vec<Annotation>,
    },
    /// `statemachine <name> { use… initial <s> state… exit… budget N steps … }` —
    /// a first-class, importable, provenance-carrying **control-flow** declaration
    /// for long-horizon procedural reasoning (triage → work-up → decision;
    /// titrate-until-target). ADJ-STATEMACHINE RS-3, `code/specs/ADJ-STATEMACHINE.md`
    /// §2 (grammar) + §5 (lowering).
    ///
    /// A sibling of [`Statement::Table`]/[`Statement::Formulabook`]: like them it is
    /// a named top-level declaration that **lowers onto the existing engine** (a
    /// guard is an ordinary predicate/compute evaluation, an action an assertion
    /// into the KB). RS-3b (this slice) parses + lowers the STRUCTURE to provenanced
    /// records; the *driver* that sequences transitions and guarantees termination
    /// is RS-3c (§3–§4). Field order mirrors the declared surface order: the
    /// imported vocabulary (`uses`), the required `initial` state, the `states`, the
    /// (required, ≥ 1) `exits`, the step `budget`, then the shared provenance
    /// envelope (`annotations`; a shipped machine must be sourced, exactly like a
    /// `table`).
    StateMachine {
        name: String,
        uses: Vec<String>,
        initial: String,
        states: Vec<StateDef>,
        exits: Vec<ExitDef>,
        budget: u64,
        annotations: Vec<Annotation>,
    },
    /// `argument <name> { premise… infer… }` — a byte-grounded **argument graph**
    /// (ADJ-ARGUMENT-IR, `code/specs/ADJ-ARGUMENT-IR.md` §2/§6; ADR-2). Decomposes a
    /// piece of prose into named [`ArgPremise`]s (asserted propositions) and
    /// [`ArgInference`] steps (a conclusion derived `from` earlier premises/inferences).
    ///
    /// A sibling of [`Statement::Table`]/[`Statement::StateMachine`], but it **lowers
    /// away entirely** into existing constructs (§2.3): each premise → a provenanced
    /// [`Statement::Relate`]-style `Fact`; each inference → a [`Statement::Rule`]-style
    /// `Rule` whose body is the terms it cites `from`. The engine then *derives* the
    /// thesis (a trailing `? thesis(…)` query), `--explain` renders the chain, and
    /// `adj-verify` re-checks it — all for free, because the argument IS adj-lang. No
    /// argument-specific evaluator, node table, or renderer exists past lowering.
    Argument {
        name: String,
        premises: Vec<ArgPremise>,
        inferences: Vec<ArgInference>,
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
    /// The multi-step `let`-bindings that precede the final body, in source
    /// order (ADJ-RULE-SUBSTRATE RS-2). Empty for the single-expression sugar
    /// (`formula f(...) = <expr>`, the rung-0 form). For the block form
    /// (`formula f(...) { let s1 = e1  let s2 = e2  <body> }`) each entry names
    /// an intermediate value; a later step and the [`body`](Self::body) may
    /// reference an earlier step's name in addition to the [`params`](Self::params).
    /// The lowerer desugars these into `body` by in-order substitution, so the
    /// RS-1 apply/expand pipeline consumes a single effective expression.
    pub steps: Vec<FormulaStep>,
    /// The formula body — the EXISTING `let` expression AST, reused verbatim.
    /// In the block form this is the final expression after the `let`-steps.
    pub body: ExprAst,
    /// The provenance envelope (`source` / `locator` / `trust`, plus any
    /// corroborating `cites`), reusing the shared [`Annotation`] set every
    /// grounded clause carries. Lowered via `annotations_to_provenance`; a
    /// shipped formula must carry a non-empty `source`.
    pub annotations: Vec<Annotation>,
}

/// A single `let`-step inside a multi-step formula body (ADJ-RULE-SUBSTRATE
/// RS-2). `let <name> = <expr>` names an intermediate value; the `expr` may
/// reference the formula's parameters and any earlier step's name. The lowerer
/// folds the steps into the final body by in-order substitution.
#[derive(Debug, Clone, PartialEq)]
pub struct FormulaStep {
    /// The step's binding name (the intermediate value's identifier).
    pub name: String,
    /// The step's defining expression (over params + earlier step names).
    pub expr: ExprAst,
}

/// One `row (v1, …, vn)` of a [`Statement::Table`] (ADJ-TABLES RS-5). The cells
/// are positionally bound to the table's declared `columns`, and the row lowers
/// to a ground relation `<table>(v1, …, vn)` carrying the table's provenance.
#[derive(Debug, Clone, PartialEq)]
pub struct TableRow {
    /// The row's cells, in column order.
    pub cells: Vec<TableCell>,
    /// This row's OWN provenance block (ADJ-TABLES RS-5e), if it wrote one. Empty
    /// for a row that inherits the table's envelope wholesale (the pre-RS-5e
    /// behaviour, and still the common case for a table whose every row is
    /// defended by the same sentence).
    ///
    /// When non-empty these annotations override the table envelope **field by
    /// field** for this row — so a row can supply just the `source` span that
    /// defends *it* and keep the table's shared `locator`/`trust`. The point is
    /// that a lookup answer cites the span supporting **the row it selected**,
    /// not the table's first sentence: with one envelope, a six-band table made
    /// every answer in every band quote the same cell, which is an accounting
    /// error the moment the selected row is explicit (as a range lookup makes it).
    pub annotations: Vec<Annotation>,
}

/// One `premise <name> : <kind> <claim> { … }` of a [`Statement::Argument`]
/// (ADJ-ARGUMENT-IR §2.1). An asserted proposition the argument starts from; it
/// lowers to a provenanced ground `Fact` (§2.3), and its `name` is how later
/// [`ArgInference`]s cite it in their `from` list. The `annotations` are the same
/// `source`/`locator`/`trust` envelope every grounded clause carries — the byte
/// citation the ADR-3 grounding gate will check.
#[derive(Debug, Clone, PartialEq)]
pub struct ArgPremise {
    /// The local name this premise binds, referenced by an inference's `from`.
    pub name: String,
    /// `extracted` (stated in the source) | `imported` (an external cited premise) |
    /// `inferred` (a hedged reading). Validated against that closed set by the lowerer
    /// (an unknown kind is a clean `LowerError`, never silently accepted). At ADR-2 the
    /// kind is recorded for the audit; the strict per-kind grounding is ADR-3.
    pub kind: String,
    /// The proposition itself, as an ADJ term — lowers to the fact's edge.
    pub claim: Term,
    /// Cite/source provenance (`source`/`locator`/`trust`).
    pub annotations: Vec<Annotation>,
}

/// One `infer <name> : <connective> conclude <conclusion> from <refs> { … }` of a
/// [`Statement::Argument`] (ADJ-ARGUMENT-IR §2.1). A step that derives `conclusion`
/// from the premises/inferences it names in `from`; it lowers to a `Rule` whose head
/// is `conclusion` and whose body is those referenced terms (§2.3), so the engine
/// chains the steps by SLD resolution. The `connective` is the open-vocabulary word as
/// written (because/therefore/suggests/…); the `annotations` carry the warrant.
#[derive(Debug, Clone, PartialEq)]
pub struct ArgInference {
    /// The local name this inference's conclusion binds, referenced by a later `from`.
    pub name: String,
    /// The open-vocabulary inference connective as written in the source.
    pub connective: String,
    /// The derived proposition — lowers to the rule head.
    pub conclusion: Term,
    /// Names of the premises/inferences this step derives its conclusion `from`. Each
    /// must resolve to an earlier `name` in the same argument (an unknown reference is a
    /// clean `LowerError`); the referenced terms become the rule's body literals.
    pub from: Vec<String>,
    /// ADJ-ARGUMENT-REBUTTAL AR-3: an optional `unless <defeater> { , <defeater> }`
    /// UNDERCUT guard. Each defeater lowers to a `not <term>` body literal
    /// (negation-as-failure), so the step fires only while its warrant is not defeated.
    /// Empty = no undercut. Unlike `from`, these are RAW terms (the defeater proposition,
    /// derived elsewhere), not references to sibling premise/inference names.
    pub unless: Vec<Term>,
    /// ADJ-ARGUMENT-REBUTTAL AR-3: the optional trailing `context: <name>` — the CONTEXT
    /// this inference is grounded in (mirroring [`Statement::Rule`]'s `context`). With a
    /// `functional` thesis + a `context_order`, a rival inference in an outranking context
    /// REBUTS (defeats) this one. `None` = context-free. Lowers to `Rule::with_context`.
    pub context: Option<String>,
    /// Warrant/source provenance (`source`/`locator`/`trust`).
    pub annotations: Vec<Annotation>,
}

/// A single table cell (ADJ-TABLES RS-5). One of the three GROUND term kinds the
/// engine stores — a number, an atom, or a string — mapped 1:1 onto
/// `logic_core::Term::{Num, Atom, Str}` by the lowerer. (A cell is never a
/// variable or a compound: a table holds ground data, not open goals.)
#[derive(Debug, Clone, PartialEq)]
pub enum TableCell {
    /// A numeric literal (`2.54`) — lowers to `logic_core::Term::Num`. Carried as a
    /// [`NumLit`] so a high-precision table cell (π to 39 places) keeps every digit
    /// through parse → store → query; the lowerer maps `Int` → `Number::Int` and
    /// `Exact` → `Number::Exact`. This is the cell kind a looked-up value flows from
    /// into a `let`/`formula`.
    Number(NumLit),
    /// A bare identifier (`inch`) — lowers to `logic_core::Term::Atom`. Typically
    /// the key column of a key→value table.
    Atom(String),
    /// A quoted string (`"mg/dL"`) — lowers to `logic_core::Term::Str`. For label
    /// cells that are not identifiers.
    Text(String),
}

/// One `state <name> { transition… }` of a [`Statement::StateMachine`]
/// (ADJ-STATEMACHINE §2). A state is a labelled node in the control-flow graph;
/// its `transitions` are the outgoing edges, tried in **source order** by the
/// driver (first-guard-wins, §3). A state with no transition is a legal dead end
/// (the driver reports `Stuck` there unless an exit criterion holds).
#[derive(Debug, Clone, PartialEq)]
pub struct StateDef {
    /// The state's name — the label a `transition … to <name>` and the machine's
    /// `initial <name>` refer to. Every such reference must name a declared state
    /// (a [`crate::LowerError::SmUnknownState`] otherwise).
    pub name: String,
    /// The outgoing transitions, in source order (the driver's selection order).
    pub transitions: Vec<TransitionDef>,
}

/// One `transition on <guard> to <target> [ do <action> { , <action> } ]` of a
/// [`StateDef`] (ADJ-STATEMACHINE §2). When its [`guard`](Self::guard) holds the
/// driver moves to [`target`](Self::target), first applying each of its
/// [`actions`](Self::actions) (each a provenanced trace step, §3).
#[derive(Debug, Clone, PartialEq)]
pub struct TransitionDef {
    /// The firing condition — a presence guard (a bare finding atom) or a numeric
    /// comparison over a slot (see [`SmGuard`]).
    pub guard: SmGuard,
    /// The state to move to when the guard fires (must be a declared state).
    pub target: String,
    /// The actions run on firing, in source order — the RS-3b minimal subset is
    /// `assert <term>` (see [`SmAction`]); `let`-binding actions are a follow-up.
    pub actions: Vec<SmAction>,
}

/// One `exit when <guard> yield <expr>` of a [`Statement::StateMachine`]
/// (ADJ-STATEMACHINE §2). When any exit's [`guard`](Self::guard) holds the run
/// halts, yielding [`yield_expr`](Self::yield_expr)'s value (with its derivation
/// trace, §4 `Halted`). A machine must declare at least one exit
/// ([`crate::LowerError::SmMissingExit`] otherwise) so the run can terminate on a
/// criterion rather than only on the step budget.
#[derive(Debug, Clone, PartialEq)]
pub struct ExitDef {
    /// The exit criterion (checked before any transition each step, §3).
    pub guard: SmGuard,
    /// The value yielded on halt — an ordinary [`ExprAst`], lowered through the
    /// same compute path as a `let`/`formula` body.
    pub yield_expr: ExprAst,
}

/// A state-machine **guard** — `( apply | IDENT ) [ relop expr ]`
/// (ADJ-STATEMACHINE §2, the RS-3b minimal subset). Two shapes share one type:
///
/// - **Presence guard** (`comparison == None`): a bare finding atom (or, in the
///   full target, a formula application). It holds iff that fact is present in the
///   KB — e.g. `transition on dose_changed to check`.
/// - **Comparison guard** (`comparison == Some((op, rhs))`): a numeric comparison
///   of the [`subject`](Self::subject)'s valued slot against `rhs` — e.g.
///   `transition on inr < 2 to increase_dose`.
///
/// [`subject`](Self::subject) is carried as an [`Term`] (an [`Term::Atom`] for a
/// bare IDENT, a [`Term::Compound`] for an `apply`) so a guard lowers through the
/// same term/compute forms the rest of the language uses — no parallel evaluator.
#[derive(Debug, Clone, PartialEq)]
pub struct SmGuard {
    /// The thing being tested: a bare slot/finding atom, or a formula application.
    pub subject: Term,
    /// The optional numeric comparison. `None` is a presence guard; `Some((op,
    /// rhs))` compares the subject's value against `rhs` with `op`.
    pub comparison: Option<(CmpOp, ExprAst)>,
}

/// One transition **action** (ADJ-STATEMACHINE §2). The RS-3b minimal subset is
/// `assert <term>` — emit a fact into the `KnowledgeBase`; `let <name> = <expr>`
/// binding actions are a follow-up (deferred, never silently dropped). Kept an
/// `enum` so that follow-up adds a variant rather than reshaping this type.
#[derive(Debug, Clone, PartialEq)]
pub enum SmAction {
    /// `assert <term>` — assert a ground fact into the KB when the transition fires.
    Assert(Term),
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
    /// `quote "<text>" at <byte_offset> snapshot "<sha256-hex>"`
    /// (RS-4 PR-D4, `ADJ-REASON-MATH.md` §E.3.1) — a **pinned verbatim span**:
    /// the exact bytes `text` must occupy at `byte_offset` in the document whose
    /// SHA-256 is `snapshot_hex`. Populates `Provenance::quote` +
    /// `Provenance::snapshot`, and is what lets `adj-verify` report
    /// `fully_verified`. The `byte_offset` is emitted by the spider at ingest —
    /// never authored by a human or model (`feedback_no_byte_arithmetic_for_llm`).
    Quote {
        text: String,
        byte_offset: usize,
        snapshot_hex: String,
    },
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
