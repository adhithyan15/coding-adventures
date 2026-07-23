//! # Lowering — AST → KnowledgeBase + queries.
//!
//! Translates the AST produced by [`crate::parser`] into a
//! `logic-engine` [`KnowledgeBase`] populated with [`Fact`],
//! [`PriorClause`], [`ContributionClause`], and
//! [`JointContributionClause`] entries. Queries appear in the
//! returned [`LoweredProgram::queries`] vector in source order; the
//! caller decides whether to run them via
//! [`logic_engine::search`] and what to do with the results.
//!
//! ## What the lowerer enforces beyond the parser
//!
//! - At most one `prior` per conclusion. Two priors for the same
//!   atom is a [`LowerError::DuplicatePrior`] (mirroring the
//!   engine's `KbError::ConflictingPriors`, but caught at lowering
//!   time so the diagnostic carries surface line/col).
//! - All three trust-tier names map to the engine's
//!   [`logic_engine::TrustTier`] variants.
//! - `source` may appear at most once per statement; multiple
//!   `source` annotations are a [`LowerError::DuplicateAnnotation`].

use logic_core::{
    atom as core_atom, compound, int as core_int, var as core_var, LogicVar, Number as CoreNumber,
    Term as CoreTerm,
};
use logic_engine::{
    compute, BodyLiteral, Citation, CmpOp as EngineCmpOp, ComputeExpr, ComputeOp, ContentHash,
    ContributionClause, Fact, JointContributionClause, KbError, KnowledgeBase,
    PredicateContributionClause, PriorClause, Priority, Provenance, Rule, TrustTier,
    UncertaintyMarker,
};

use std::collections::HashMap;

use crate::ast::{
    AggOp, Annotation, ArithOp, BinFn, CmpOp, Define, DefineKind, Evidence, ExprAst, FormulaDef,
    NamedFn, NumLit, OptDir, Program, RelOp, Statement, Term as AstTerm, TrustTierName,
};

/// One lowered constraint: `lhs <op> rhs`, with both sides kept as
/// **unevaluated** [`ComputeExpr`] trees (they reference symbols the solver
/// will assign, so they cannot be computed yet — that is the solver's job in
/// track B2).
#[derive(Debug, Clone, PartialEq)]
pub struct LoweredConstraint {
    pub lhs: ComputeExpr,
    pub op: RelOp,
    pub rhs: ComputeExpr,
}

/// The typed constraint system a program builds from its `symbol` /
/// `constrain` / `solve for` / `check` statements (ADJ constraints track B).
/// Track B1 builds and exposes it; the solver backends are wired in B2.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConstraintSystem {
    /// Declared unknowns: `(name, sort)`, where `sort` is a dimensional sort
    /// term (`scalar`, `money(usd)`, …).
    pub symbols: Vec<(String, CoreTerm)>,
    /// The asserted (in)equalities.
    pub constraints: Vec<LoweredConstraint>,
    /// The unknowns a `solve for { … }` asked to solve.
    pub solve_for: Vec<String>,
    /// Whether a `check` (feasibility query) was requested.
    pub check: bool,
    /// An optional `minimize`/`maximize` objective (ADJ constraints track C2):
    /// `(direction, objective expression)`. The expression is kept unevaluated
    /// (it mentions the symbols the LP solver will assign).
    pub objective: Option<(OptDir, ComputeExpr)>,
}

impl ConstraintSystem {
    /// `true` iff the program declared no constraint machinery at all (the
    /// common case for a pure prior/contributes rulebook).
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
            && self.constraints.is_empty()
            && self.solve_for.is_empty()
            && !self.check
            && self.objective.is_none()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LowerError {
    DuplicatePrior {
        conclusion_name: String,
    },
    DuplicateAnnotation {
        name: &'static str,
    },
    /// A `quote … at … snapshot …` pin (RS-4 PR-D4, §E.3.1) was malformed: the
    /// snapshot was not a 64-char SHA-256 hex, or the quoted text carried no
    /// visible content to anchor. Fail-closed — a malformed pin is a compile
    /// error, never a half-built `Verbatim` span the verifier would then reject.
    MalformedQuotePin {
        reason: &'static str,
    },
    EngineRejected {
        detail: String,
    },
    /// A `prior <p>` whose probability is not in the open interval
    /// `(0.0, 1.0)`. The engine's `PriorClause::from_probability`
    /// asserts this; we catch it at lowering time so a malformed
    /// rulebook produces a clean diagnostic instead of a process panic.
    InvalidProbability {
        value: f64,
    },
    /// A `contributes <lr>` / `interacts <lr>` whose likelihood ratio is
    /// not strictly positive and finite. LR is a ratio of probabilities,
    /// so `lr <= 0` (or non-finite) is a modeller error — rejected here
    /// rather than panicking in `from_lr`.
    InvalidLikelihoodRatio {
        value: f64,
    },
    /// A `rule { … priority: <tier> }` whose tier is not one of the named
    /// [`Priority`] levels (`default` | `specific` | `authoritative` | `mandatory`).
    /// Caught at lowering so a typo produces a clean diagnostic, not a silent default.
    UnknownPriorityTier {
        tier: String,
    },
    /// A `let <name> = <expr>` whose formula could not be evaluated (an
    /// unknown slot, division by zero, an empty aggregation, …). Carries
    /// the engine's [`logic_engine::ComputeError`] rendered for the audit.
    ComputationFailed {
        name: String,
        detail: String,
    },
    /// A finding / hypothesis term used in a clause is not `define`d in the
    /// program's dictionary (MYCIN-2026 closed-vocabulary enforcement).
    /// `expected` is the role the term was used in (`"finding"`/`"hypothesis"`).
    UndefinedTerm {
        name: String,
        expected: &'static str,
    },
    /// A finding term used a value outside its declared `values [...]` domain.
    ValueNotInDomain {
        functor: String,
        value: String,
        domain: Vec<String>,
    },
    /// A `use <name>` (M2) named a dictionary that the program never declared.
    UndefinedDictionary {
        name: String,
    },
    /// A `rulebook` nested inside another `rulebook` (MYCIN-2026 M2). A rulebook
    /// is a *flat* container; nested rulebooks have no defined
    /// vocabulary-scoping semantics, so they are rejected cleanly rather than
    /// silently mis-scoped. (Also caps `flatten_clauses` recursion at one level,
    /// closing an unbounded-recursion DoS on untrusted source.)
    NestedRulebook {
        outer: String,
        inner: String,
    },
    /// A `formula`'s body referenced a free identifier that is not one of its
    /// declared parameters (ADJ-FORMULA-LIBRARIES rung-0 parameter-scoping). A
    /// formula is a *closed* parameterized expression — every leaf must name a
    /// parameter — so a stray identifier is a clean compile error, never a silent
    /// mis-binding to some observed slot at apply time. Carries the offending
    /// formula and variable.
    FormulaFreeVariable {
        formula: String,
        variable: String,
    },
    /// A shipped `formula` carried no `source` (the provenance-required lint,
    /// mirroring the recall-library adversarial write gate). A formula is a *claim
    /// about the world* ("BMI is mass ÷ height²") and may not enter a library
    /// unsourced — humans correct provenance, they do not author formulas by fiat.
    FormulaMissingProvenance {
        formula: String,
    },
    /// A `? name(args)` applied a `formula` whose argument was neither a plain
    /// identifier (bind a parameter to a like-named slot) nor a number literal.
    /// A compound / variable argument has no meaning as a formula binding at
    /// rung-0, so it is rejected rather than silently mis-applied.
    FormulaBadArgument {
        formula: String,
    },
    /// A formula application `name(args)` in an expression named a formula that is
    /// not registered by any imported `formulabook` (ADJ-RULE-SUBSTRATE RS-1). A
    /// bare `IDENT` with no parentheses is an ordinary slot reference and does NOT
    /// reach here; only an `IDENT(...)` application whose callee is unknown does.
    /// Distinct from the aggregation/built-in-call paths (`sum(slot)`, `\sin(x)`),
    /// which are recognised earlier — so an unknown *formula* is a clean, specific
    /// diagnostic, never confused with a mistyped aggregation.
    FormulaUnknown {
        name: String,
    },
    /// A formula application supplied the wrong number of arguments for the named
    /// formula (ADJ-RULE-SUBSTRATE RS-1). Carries the formula, the arity it
    /// declares, and the count it was applied with.
    FormulaArity {
        formula: String,
        expected: usize,
        got: usize,
    },
    /// A formula application expanded deeper than [`FORMULA_MAX_APPLY_DEPTH`]
    /// (ADJ-RULE-SUBSTRATE RS-1). A self- or mutually-recursive formula
    /// (`formula f(x) = f(x)`) would otherwise expand forever; the depth guard
    /// turns it into this clean, typed error — the compute analogue of the
    /// resolver's recursion guard — so the process abstains with a reason instead
    /// of hanging or overflowing the stack. Carries the depth limit reached.
    FormulaRecursionTooDeep {
        formula: String,
        limit: usize,
    },
    /// A formula application expanded into more than [`FORMULA_MAX_EXPANSION_NODES`]
    /// total AST nodes (ADJ-RULE-SUBSTRATE RS-1). **Depth alone cannot bound size:**
    /// a body that references a parameter more than once (`formula g(x) = x * x`)
    /// duplicates the bound argument subtree on every substitution, so a chain of
    /// such formulas (`pᵢ(x) = pᵢ₋₁(x) * pᵢ₋₁(x)`) grows the expanded expression as
    /// `2ⁿ` while the recursion depth grows only linearly — the depth guard never
    /// fires and the process OOMs. This node budget is charged on every substituted
    /// / emitted node and bails BEFORE the exponential tree is materialised, turning
    /// an adversarial formulabook into a clean, fast, typed error. The cap is
    /// generous for any legitimate composed formula (a handful of applications) yet
    /// a tiny fraction of the blow-up. Carries the node limit reached.
    FormulaExpansionTooLarge {
        limit: usize,
    },
    /// A single expression nested deeper than [`FORMULA_MAX_NODE_DEPTH`] AST levels
    /// (ADJ-RULE-SUBSTRATE RS-1). Distinct from [`FormulaExpansionTooLarge`], which
    /// bounds total *size*: a left-leaning operator spine (`x + 0 + 0 + … + 0`, a
    /// thousand terms) is small in nodes-per-level but arbitrarily DEEP, and the
    /// recursive expansion/substitution walkers descend it frame-for-frame. Without a
    /// depth bound a crafted deep spine overflows the native stack (an abort, not a
    /// catchable error) before the node budget — which only trips at the BOTTOM of the
    /// descent — ever fires. This guard caps walker recursion at [`FORMULA_MAX_NODE_DEPTH`]
    /// (the stack-safe bound `adapter` uses for the same reason), so a pathological
    /// spine is a clean, typed error long before it can exhaust the stack. It never
    /// rejects a legitimate expression: those are a handful of levels deep, and
    /// anything past the evaluator's own `MAX_EVAL_DEPTH` it would reject anyway.
    /// Carries the depth limit reached.
    FormulaNestingTooDeep {
        limit: usize,
    },
    /// A `table` row's cell count did not match the table's declared `columns`
    /// (ADJ-TABLES RS-5). A table's arity is fixed by its `columns` clause; a row
    /// of a different length would lower to a relation of the wrong arity (and so
    /// never match a lookup, or silently shadow a real one), so it is a clean
    /// compile error instead. Carries the table name, the declared column count,
    /// the offending row's index (0-based), and its actual cell count.
    TableArity {
        table: String,
        expected: usize,
        row: usize,
        got: usize,
    },
    /// A shipped `table` carried no `source` (the provenance-required lint, shared
    /// with `formula`/`relate`). A table asserts facts about the world — the very
    /// reason it is a first-class construct is to be the auditable home for a
    /// *cited* published table — so it may not enter a library unsourced. Carries
    /// the table name.
    TableMissingProvenance {
        table: String,
    },
    /// A `table` declared zero `columns` (ADJ-TABLES RS-5). A table with no columns
    /// has no arity and no lookup key; it is almost certainly a mistake, so it is
    /// rejected rather than lowered to a stream of nullary facts. Carries the name.
    TableNoColumns {
        table: String,
    },
    /// A `? lookup <table> …` named a table that was never declared (ADJ-TABLES
    /// RS-5c). A range lookup reads a specific `table` as a step function; an
    /// unknown name would silently abstain forever, so it is a clean compile
    /// error. Carries the referenced name.
    LookupUnknownTable {
        table: String,
    },
    /// A `? lookup <table> <col> …` named a `col` that is not one of the table's
    /// declared `columns` (ADJ-TABLES RS-5c) — either the key column or the
    /// `give` value column. Carries the table and the offending column name.
    LookupUnknownColumn {
        table: String,
        column: String,
    },
    /// A range lookup's key column holds a non-numeric cell (ADJ-TABLES RS-5c). A
    /// `range` lookup compares the query value against the key column with the
    /// exact numeric comparators, so every key cell must be a number; an atom or
    /// string key is meaningless for a step function. Carries the table, the key
    /// column, and the offending row index (0-based).
    LookupNonNumericKeyColumn {
        table: String,
        column: String,
        row: usize,
    },
    /// A lookup named `mode <name>` for a mode that is spec'd but not yet built
    /// (ADJ-TABLES RS-5d) — today only `interpolated`. Rejected explicitly (rather
    /// than silently treated as `range`) so the reserved surface is honest about
    /// what the engine can do. Carries the requested mode.
    LookupModeUnsupported {
        mode: String,
    },
    /// A lookup named a `mode` that is not a recognized tactic at all (ADJ-TABLES
    /// RS-5c). The valid modes are `range` (built) and `interpolated` (reserved,
    /// RS-5d). Carries the unrecognized mode.
    LookupUnknownMode {
        mode: String,
    },
    /// An `import "<path>"` survived to lowering (MYCIN-2026 M3). Imports must be
    /// resolved by [`crate::resolve`] *before* `lower` runs — reaching here means
    /// the caller used `compile` directly on a program that still has imports,
    /// instead of the import-resolving entry point. Rejected so an `import` is
    /// never silently dropped.
    UnresolvedImport {
        path: String,
    },
}

/// One lowered RANGE / BRACKET lookup (ADJ-TABLES RS-5c) — the validated,
/// index-resolved form of a `? lookup <table> <key_col> = <n> mode range give
/// <value_col>` recall. The lowerer has already checked the table and columns
/// exist and that the key column is numeric, so the runtime tactic (in the CLI)
/// only has to: read the table's ground facts, convert the key cells and the
/// query value to exact rationals, select the row whose key is the greatest
/// `<= key_value`, and return its value cell **with that row's citation** (or
/// abstain when the query is below the table's domain). Nothing here is `f64`;
/// the comparison rides the engine's exact `CmpOp`/`BigRational` path.
#[derive(Debug, Clone)]
pub struct LoweredRangeLookup {
    /// The table relation's functor (also the fact functor to scan).
    pub table: String,
    /// The relation arity (= the table's column count) — the fact arity to match.
    pub arity: usize,
    /// The 0-based position of the key column among the table's columns.
    pub key_index: usize,
    /// The 0-based position of the `give` value column among the table's columns.
    pub value_index: usize,
    /// The key column's declared name (for the answer's audit trail).
    pub key_col: String,
    /// The value column's declared name (the binding name in the answer).
    pub value_col: String,
    /// The concrete query value to classify, as a ground `Num` term (exact — no
    /// digit lost) so the comparison is done on the exact numeric path.
    pub key_value: CoreTerm,
    /// The tactic named at the call site — `range` (the only mode this lowers).
    pub mode: String,
}

/// The result of lowering — a populated KB, any queries to run, and the
/// (possibly empty) constraint system the program declared.
#[derive(Debug)]
pub struct LoweredProgram {
    pub kb: KnowledgeBase,
    pub queries: Vec<CoreTerm>,
    /// The RANGE / BRACKET lookups the program declared (ADJ-TABLES RS-5c),
    /// resolved to relation + column indices + exact query value. Empty for a
    /// program with no `? lookup … mode range …`. Run by the CLI's range tactic.
    pub range_lookups: Vec<LoweredRangeLookup>,
    pub constraints: ConstraintSystem,
    /// The controlled vocabulary the program declared (MYCIN-2026): the
    /// `define`d findings + hypotheses, with their surface forms. Empty for a
    /// program with no dictionary. Used by the decomposer (surface forms) and
    /// enforced at compile time.
    pub dictionary: Vec<Define>,
}

/// Lower an [`ast::Program`] to a populated KB + queries + constraint system.
pub fn lower(program: &Program) -> Result<LoweredProgram, LowerError> {
    let mut kb = KnowledgeBase::new();
    let mut queries = Vec::new();
    let mut constraints = ConstraintSystem::default();
    let mut dictionary: Vec<Define> = Vec::new();
    // ADJ-RULE-SUBSTRATE RS-1: `contributes … from <formula-app> <op> <thr>` clauses
    // whose gated LHS is a FORMULA APPLICATION are collected here and lowered in a
    // second pass, after every statement (and therefore every `observe`) has been
    // seen — see the `Evidence::Predicate` arm for why a library's branch precedes
    // its consumer's observations.
    let mut deferred_formula_predicates: Vec<DeferredFormulaPredicate> = Vec::new();

    // Flatten rulebooks (MYCIN-2026 M2): a `rulebook <name> { … }` is a named
    // container whose clauses lower into the KB exactly as if written at top
    // level — the name is metadata for import/addressing (M3), not a separate
    // namespace. The flat list drives KB building; the original nested
    // structure is used afterward to scope vocabulary enforcement per `use`.
    let mut flat: Vec<&Statement> = Vec::new();
    flatten_clauses(&program.statements, &mut flat)?;

    // ADJ-FORMULA-LIBRARIES rung-0: register every `formula` from every
    // `formulabook` into a name→definition map, up front (formulas are
    // order-independent declarations). Each is validated as it is registered:
    // parameter-scoping (no free identifier in the body) and the
    // provenance-required lint (a shipped formula must be sourced). A later
    // `? name(args)` looks the name up here and APPLIES it — a named, importable,
    // reusable `let` (see the `Statement::Query` arm below).
    let mut formulas: HashMap<&str, &FormulaDef> = HashMap::new();
    for stmt in flat.iter().copied() {
        if let Statement::Formulabook { formulas: defs, .. } = stmt {
            for fd in defs {
                validate_formula(fd)?;
                formulas.insert(fd.name.as_str(), fd);
            }
        }
    }

    // ADJ-TABLES RS-5c: register every `table`'s columns + rows into a name→table
    // map, up front (like the formula map — tables are order-independent, and a
    // `? lookup` may precede the table it reads, or read an imported one). A range
    // lookup validates its table/columns against this map and needs the rows to
    // check the key column is numeric.
    #[allow(clippy::type_complexity)]
    let mut tables: HashMap<&str, (&[String], &[crate::ast::TableRow])> = HashMap::new();
    for stmt in flat.iter().copied() {
        if let Statement::Table {
            name,
            columns,
            rows,
            ..
        } = stmt
        {
            tables.insert(name.as_str(), (columns.as_slice(), rows.as_slice()));
        }
    }
    // ADJ-TABLES RS-5c: the validated range/bracket lookups, run by the CLI tactic.
    let mut range_lookups: Vec<LoweredRangeLookup> = Vec::new();

    for stmt in flat.iter().copied() {
        match stmt {
            Statement::Prior {
                probability,
                conclusion,
                annotations,
            } => {
                // Guard the engine's `from_probability` assertion: a
                // probability outside (0, 1) would panic. Reject it as a
                // clean lowering error instead.
                if !(probability.is_finite() && *probability > 0.0 && *probability < 1.0) {
                    return Err(LowerError::InvalidProbability {
                        value: *probability,
                    });
                }
                let prov = annotations_to_provenance(annotations)?;
                let clause = PriorClause::from_probability(lower_term(conclusion), *probability)
                    .with_provenance(prov);
                kb.add_prior(clause).map_err(|e| match e {
                    KbError::ConflictingPriors { conclusion, .. } => LowerError::DuplicatePrior {
                        conclusion_name: format!("{conclusion:?}"),
                    },
                })?;
            }
            Statement::Contributes {
                lr,
                evidence,
                conclusion,
                annotations,
            } => {
                check_lr(*lr)?;
                let prov = annotations_to_provenance(annotations)?;
                match evidence {
                    // Ordinary term evidence → single-source LR contribution.
                    Evidence::Term(t) => {
                        let clause =
                            ContributionClause::from_lr(lower_term(conclusion), lower_term(t), *lr)
                                .with_provenance(prov);
                        kb.add_contribution(clause);
                    }
                    // Numeric predicate evidence → predicate-gated contribution.
                    // The comparison is evaluated on the CPU at decision time;
                    // a saturating `lr` makes the rule deterministic.
                    Evidence::Predicate { lhs, op, rhs } => match lhs {
                        // The ordinary case: the LHS is a bare slot identifier — an
                        // observed value or a `let`-derived one. Gate on it directly;
                        // the comparison runs on the CPU at decision time (a
                        // saturating `lr` makes the rule deterministic). This slot is
                        // read at DECISION time, so it needn't exist yet.
                        ExprAst::Ref(slot) => {
                            let clause = PredicateContributionClause::from_lr_expr(
                                lower_term(conclusion),
                                slot.clone(),
                                lower_cmp_op(*op),
                                lower_expr(rhs),
                                *lr,
                            )
                            .with_provenance(prov);
                            kb.add_predicate_contribution(clause);
                        }
                        // RS-1 BRANCH ON A FORMULA: the LHS is a formula application
                        // (`bmi(body_mass, height) >= 30`). The formula reads observed
                        // slots (`body_mass`/`height`) that a *consumer* supplies —
                        // and, because a library declares the branch while the consumer
                        // `observe`s the numbers, those observations arrive AFTER this
                        // clause in lowering order. So we DEFER: record the branch now
                        // and compute the formula once the KB is fully populated (see
                        // the deferred-predicate pass after the statement loop). The
                        // formula becomes a derived slot; the predicate then gates on
                        // it exactly like any other derived value.
                        _ => {
                            deferred_formula_predicates.push(DeferredFormulaPredicate {
                                lhs: lhs.clone(),
                                op: *op,
                                rhs: rhs.clone(),
                                lr: *lr,
                                conclusion: lower_term(conclusion),
                                prov,
                            });
                        }
                    },
                }
            }
            Statement::Interacts {
                lr,
                evidence_set,
                conclusion,
                annotations,
            } => {
                check_lr(*lr)?;
                let prov = annotations_to_provenance(annotations)?;
                let clause = JointContributionClause::from_lr(
                    lower_term(conclusion),
                    evidence_set.iter().map(lower_term).collect(),
                    *lr,
                )
                .with_provenance(prov);
                kb.add_joint_contribution(clause);
            }
            Statement::Observe { term } => {
                kb.add_fact(Fact::certain(lower_term(term)));
            }
            Statement::Relate { edge, annotations } => {
                // A ground relational edge → a Certain Fact carrying its citation
                // as provenance, so a binding query's answer can name the edge
                // (and its source) as its proof. The edge term's functor is the
                // relation; its arguments are the entities.
                let prov = annotations_to_provenance(annotations)?;
                kb.add_fact(Fact::certain(lower_term(edge)).with_provenance(prov));
            }
            Statement::Functional { functor, arity } => {
                // ADJ73 PR-C: declare the predicate functional on its last argument, so
                // conflicting derivations are resolved by precedence (enumerate_governing).
                kb.declare_functional(functor, *arity);
            }
            Statement::ContextOrder { edges } => {
                // ADJ73 PR-B: each `a > b` edge asserts that context `a` outranks context `b`
                // (lex superior) — consulted before the priority tier in resolution.
                for (higher, lower) in edges {
                    kb.add_context_outranks(higher.clone(), lower.clone());
                }
            }
            Statement::Rule {
                head,
                body,
                annotations,
                priority,
                context,
            } => {
                // A derivation rule → `logic_engine::Rule { head, body }`. Head and body
                // share ONE variable scope so a `$Var` in the head unifies with the same
                // `$Var` in a body literal (clause-scope, like a binding query). `not <lit>`
                // lowers to negation-as-failure. The citation (if grounded) rides on the
                // rule as provenance — a CAS rule stays byte-traceable.
                let mut vars = HashMap::new();
                let head_term = lower_term_scoped(head, &mut vars);
                let body_lits: Vec<BodyLiteral> = body
                    .iter()
                    .map(|lit| {
                        let t = lower_term_scoped(&lit.term, &mut vars);
                        if lit.negated {
                            BodyLiteral::Neg(t)
                        } else {
                            BodyLiteral::Pos(t)
                        }
                    })
                    .collect();
                let prov = annotations_to_provenance(annotations)?;
                // ADJ73 PR-C: map the optional named tier → Priority (absent ⇒ Default).
                let tier = match priority.as_deref() {
                    None | Some("default") => Priority::Default,
                    Some("specific") => Priority::Specific,
                    Some("authoritative") => Priority::Authoritative,
                    Some("mandatory") => Priority::Mandatory,
                    Some(other) => {
                        return Err(LowerError::UnknownPriorityTier { tier: other.into() })
                    }
                };
                // ADJ73 PR-B: the optional `context: <name>` grounds the rule in a context.
                let mut rule = Rule::certain(head_term, body_lits)
                    .with_provenance(prov)
                    .with_priority(tier);
                if let Some(ctx) = context {
                    rule = rule.with_context(ctx.clone());
                }
                kb.add_rule(rule);
            }
            Statement::Query { conclusion } => {
                // ADJ-FORMULA-LIBRARIES rung-0: is this a FORMULA APPLICATION? If the
                // goal's functor names a declared `formula` with matching arity
                // (`? bmi(body_mass, height)`), APPLY it: bind each parameter to its
                // argument, substitute into the formula body, and evaluate through the
                // SAME `ComputeExpr` evaluator a `let` uses — a formula is a named,
                // importable, reusable `let`. The result is bound as a derived value
                // named after the formula, carrying the formula's cited provenance so
                // the computed answer is auditable back to WHY its formula is trusted.
                if let Some(fd) = formula_for_query(conclusion, &formulas) {
                    let substituted = apply_formula(fd, conclusion)?;
                    // RS-1 composition: the substituted body may ITSELF apply other
                    // formulas (`ratio(n, d) = quotient(n, d)`). Expand every nested
                    // application recursively — with the depth guard — collecting the
                    // provenance chain of each applied formula, then lower the
                    // fully-expanded (application-free) expression through the SAME
                    // `compute` path a `let` uses.
                    let (expanded, chain) = expand_applies(&substituted, &formulas, 0)?;
                    let cexpr = lower_expr(&expanded);
                    let derived = compute(fd.name.clone(), &cexpr, &kb).map_err(|e| {
                        LowerError::ComputationFailed {
                            name: fd.name.clone(),
                            detail: format!("{e:?}"),
                        }
                    })?;
                    // The provenance-required lint already guaranteed a non-empty
                    // source at registration; stamp the resolved envelope onto the
                    // value — and COMPOSE the nested chain, so a value computed via
                    // `quotient` carries BOTH `ratio`'s cite (primary) and
                    // `quotient`'s (a corroboration). The derivation is auditable
                    // back to every formula that produced it.
                    let primary = annotations_to_provenance(&fd.annotations)?;
                    let prov = compose_provenance(primary, &chain);
                    kb.add_derived(derived.with_provenance(prov));
                } else {
                    // An ordinary query. Lower with a per-query variable scope so
                    // repeated `$Var`s in one goal share identity (Prolog clause-scope
                    // semantics). A ground hypothesis query lowers as before.
                    let mut vars = HashMap::new();
                    queries.push(lower_term_scoped(conclusion, &mut vars));
                }
            }
            Statement::RangeLookup {
                table,
                key_col,
                key_value,
                mode,
                value_col,
            } => {
                // ADJ-TABLES RS-5c: validate a `? lookup … mode range …` against the
                // table registry and resolve the key/value columns to positional
                // indices, so the runtime tactic never has to re-parse column names.
                let (columns, rows) =
                    tables
                        .get(table.as_str())
                        .ok_or_else(|| LowerError::LookupUnknownTable {
                            table: table.clone(),
                        })?;
                // Mode: `range` is built; `interpolated` is the reserved RS-5d
                // tactic (rejected honestly, not silently treated as range); any
                // other word is not a tactic at all.
                match mode.as_str() {
                    "range" => {}
                    "interpolated" => {
                        return Err(LowerError::LookupModeUnsupported { mode: mode.clone() })
                    }
                    _ => return Err(LowerError::LookupUnknownMode { mode: mode.clone() }),
                }
                let key_index = columns.iter().position(|c| c == key_col).ok_or_else(|| {
                    LowerError::LookupUnknownColumn {
                        table: table.clone(),
                        column: key_col.clone(),
                    }
                })?;
                let value_index = columns.iter().position(|c| c == value_col).ok_or_else(|| {
                    LowerError::LookupUnknownColumn {
                        table: table.clone(),
                        column: value_col.clone(),
                    }
                })?;
                // The key column must be numeric in EVERY row — a `range` lookup
                // compares the query against it on the exact numeric path, so an
                // atom/string key cell is a compile error (never a silent skip).
                for (i, row) in rows.iter().enumerate() {
                    if !matches!(
                        row.cells.get(key_index),
                        Some(crate::ast::TableCell::Number(_))
                    ) {
                        return Err(LowerError::LookupNonNumericKeyColumn {
                            table: table.clone(),
                            column: key_col.clone(),
                            row: i,
                        });
                    }
                }
                range_lookups.push(LoweredRangeLookup {
                    table: table.clone(),
                    arity: columns.len(),
                    key_index,
                    value_index,
                    key_col: key_col.clone(),
                    value_col: value_col.clone(),
                    key_value: lower_numlit(key_value),
                    mode: mode.clone(),
                });
            }
            Statement::Let { name, expr } => {
                // Evaluate the formula against the facts (and any earlier
                // `let`s) seen so far — statements lower in source order, so a
                // `let` sees every `observe` above it. The engine builds the
                // derivation tree; the model never computed anything.
                //
                // RS-1: a `let` may itself APPLY a library formula
                // (`let r = ratio(a, b)`); expand every nested application first,
                // collecting the applied-formula provenance chain.
                let (expanded, chain) = expand_applies(expr, &formulas, 0)?;
                let cexpr = lower_expr(&expanded);
                let derived = compute(name.clone(), &cexpr, &kb).map_err(|e| {
                    LowerError::ComputationFailed {
                        name: name.clone(),
                        detail: format!("{e:?}"),
                    }
                })?;
                // A plain `let` over observed slots has an empty chain and no
                // library provenance (its audit trail is the derivation tree). A
                // `let` that applied formulas carries their composed cites.
                let derived = match compose_chain(&chain) {
                    Some(prov) => derived.with_provenance(prov),
                    None => derived,
                };
                kb.add_derived(derived);
            }
            Statement::Uncertain {
                domain,
                conclusion,
                annotations,
            } => {
                let prov = annotations_to_provenance(annotations)?;
                let marker = UncertaintyMarker::new(
                    lower_term(conclusion),
                    domain.iter().map(lower_term).collect(),
                )
                .with_provenance(prov);
                kb.add_uncertainty_marker(marker);
            }
            // ---- constraint sublanguage (track B) ----
            Statement::Symbol { name, sort } => {
                constraints.symbols.push((name.clone(), lower_term(sort)));
            }
            Statement::Constrain { lhs, op, rhs } => {
                // Keep both sides unevaluated — they mention symbols the solver
                // will assign. lower_expr is a pure ExprAst → ComputeExpr map.
                constraints.constraints.push(LoweredConstraint {
                    lhs: lower_expr(lhs),
                    op: *op,
                    rhs: lower_expr(rhs),
                });
            }
            Statement::SolveFor { names } => {
                constraints.solve_for.extend(names.iter().cloned());
            }
            Statement::Check => {
                constraints.check = true;
            }
            Statement::Optimize { dir, objective } => {
                // Keep the objective unevaluated — it mentions the symbols the
                // LP solver assigns. A second `minimize`/`maximize` overwrites
                // the first (a program declares one objective).
                constraints.objective = Some((*dir, lower_expr(objective)));
            }
            // ---- dictionary (MYCIN-2026) ----
            Statement::Define(def) => dictionary.push(def.clone()),
            Statement::Dictionary { defines, .. } => dictionary.extend(defines.iter().cloned()),
            // ---- rulebook + use (MYCIN-2026 M2) ----
            // `use` is purely a vocabulary binding (handled in enforcement
            // below); it adds nothing to the KB. `Rulebook` never reaches here
            // — `flatten_clauses` expanded it into its inner clauses already.
            Statement::Use(_) => {}
            Statement::Rulebook { .. } => {}
            // A `formulabook` adds nothing to the KB itself — its formulas were
            // registered (and validated) in the pre-pass above, ready to be
            // APPLIED by a matching `? name(args)`. The inner `use <dict>`
            // bindings are documentation of the vocabulary the formulas are typed
            // against (rung-0 does not enforce dictionary typing of parameters).
            Statement::Formulabook { .. } => {}
            // ---- table (ADJ-TABLES RS-5) ----
            // Each row lowers to a ground relation `name(cell1, …, celln)`
            // carrying the table's provenance — byte-identical to how a `relate`
            // edge lowers (above) — so EXACT lookup is just the existing SLD
            // binding query, and a looked-up NUMBER feeds a `let`/`formula` via
            // the existing arity-1 slot/`Ref` path. Two guards keep a table honest:
            // (1) the arity of every row must match the declared `columns` (a wrong
            // arity would silently never match, or shadow, a real lookup); and
            // (2) the provenance-required lint — a shipped table must be sourced,
            // exactly like `formula`/`relate` (the whole reason a table is
            // first-class is to be the auditable home for a *cited* published table).
            Statement::Table {
                name,
                uses: _,
                columns,
                rows,
                annotations,
            } => {
                if columns.is_empty() {
                    return Err(LowerError::TableNoColumns {
                        table: name.clone(),
                    });
                }
                let prov = annotations_to_provenance(annotations)?;
                if prov.source.trim().is_empty() {
                    return Err(LowerError::TableMissingProvenance {
                        table: name.clone(),
                    });
                }
                for (i, row) in rows.iter().enumerate() {
                    if row.cells.len() != columns.len() {
                        return Err(LowerError::TableArity {
                            table: name.clone(),
                            expected: columns.len(),
                            row: i,
                            got: row.cells.len(),
                        });
                    }
                    let args: Vec<CoreTerm> = row.cells.iter().map(lower_cell).collect();
                    // ADJ-TABLES RS-5e: each row already becomes its OWN `Fact`, and
                    // every citation path (exact recall, range lookup, the proof
                    // DAG's `via_facts`) cites *the fact that produced the answer* —
                    // so stamping the ROW's provenance here is the whole fix, with no
                    // renderer change. A row that wrote its own `{ … }` block
                    // overrides the envelope field-by-field; a row without one gets
                    // the envelope unchanged (pre-RS-5e behaviour).
                    let row_prov = row_provenance(&prov, &row.annotations)?;
                    kb.add_fact(Fact::certain(compound(name, args)).with_provenance(row_prov));
                }
            }
            // ---- import (MYCIN-2026 M3) ----
            // Imports are resolved away by `crate::resolve` before lowering; one
            // reaching here means `compile` was called on an unresolved program.
            Statement::Import(path) => {
                return Err(LowerError::UnresolvedImport { path: path.clone() })
            }
        }
    }

    // ADJ-RULE-SUBSTRATE RS-1 — deferred branch-on-formula pass. Every statement
    // (and therefore every `observe`d input) is now in the KB, so we can compute
    // each formula-application predicate LHS into a derived value and gate the
    // contribution on it. This is what lets a *library* declare `contributes … from
    // bmi(body_mass, height) >= 30 to obese` and a separate *consumer* supply the
    // numbers: the branch and the observations meet here, in the completed KB.
    for d in &deferred_formula_predicates {
        // Expand the application (recursively; formula-calls-formula supported),
        // collecting its provenance chain, then compute it against the full KB.
        let (expanded, chain) = expand_applies(&d.lhs, &formulas, 0)?;
        let slot_name = match &d.lhs {
            ExprAst::Apply(name, _) => name.clone(),
            // A compound LHS expression that merely *contains* an application gets a
            // stable synthesized slot name (direct applications are the common case).
            _ => "__branch_lhs".to_string(),
        };
        let cexpr = lower_expr(&expanded);
        let derived =
            compute(slot_name.clone(), &cexpr, &kb).map_err(|e| LowerError::ComputationFailed {
                name: slot_name.clone(),
                detail: format!("{e:?}"),
            })?;
        // The derived slot carries the applied formula's composed provenance — so
        // the audit trail for a fired `obese` verdict cites BOTH the WHO obesity
        // threshold (on the contribution clause) AND the BMI definition (here).
        let derived = match compose_chain(&chain) {
            Some(prov) => derived.with_provenance(prov),
            None => derived,
        };
        kb.add_derived(derived);
        // The RHS threshold may itself apply a formula; expand before lowering.
        let (rhs_expanded, _) = expand_applies(&d.rhs, &formulas, 0)?;
        let clause = PredicateContributionClause::from_lr_expr(
            d.conclusion.clone(),
            slot_name,
            lower_cmp_op(d.op),
            lower_expr(&rhs_expanded),
            d.lr,
        )
        .with_provenance(d.prov.clone());
        kb.add_predicate_contribution(clause);
    }

    // Compile-time vocabulary enforcement (MYCIN-2026 M1 + M2). Two modes:
    //
    //   M2 (a `use` appears anywhere): enforcement is SCOPED by `use`. A
    //   top-level `use D` checks the top-level clauses against dictionary `D`;
    //   a rulebook's own `use D'` checks that rulebook against `D'` (falling
    //   back to a top-level `use`). A scope with no `use` is unchecked, and a
    //   `use` naming an undeclared dictionary is `UndefinedDictionary`. This is
    //   how a rulebook "written once" binds the vocabulary it reasons in.
    //
    //   M1 (no `use` anywhere): a declared dictionary (≥1 `define`) checks the
    //   WHOLE program against the union of all defines. Backward-compatible —
    //   programs with neither `use` nor `define` are entirely unchecked.
    if program_contains_use(&program.statements) {
        let named_dicts: std::collections::HashMap<&str, &[Define]> = program
            .statements
            .iter()
            .filter_map(|s| match s {
                Statement::Dictionary { name, defines } => {
                    Some((name.as_str(), defines.as_slice()))
                }
                _ => None,
            })
            .collect();
        let resolve = |name: &str| -> Result<&[Define], LowerError> {
            named_dicts
                .get(name)
                .copied()
                .ok_or_else(|| LowerError::UndefinedDictionary {
                    name: name.to_string(),
                })
        };

        let top_use = first_use(&program.statements);
        // Top-level group: every top-level statement that isn't a rulebook.
        if let Some(d) = top_use {
            let defs = resolve(d)?;
            let top_clauses = program
                .statements
                .iter()
                .filter(|s| !matches!(s, Statement::Rulebook { .. }));
            enforce_vocabulary(top_clauses, defs, &formulas)?;
        }
        // Each rulebook is its own scope (its `use`, else the top-level `use`).
        for s in &program.statements {
            if let Statement::Rulebook { statements, .. } = s {
                if let Some(d) = first_use(statements).or(top_use) {
                    let defs = resolve(d)?;
                    let mut clauses: Vec<&Statement> = Vec::new();
                    flatten_clauses(statements, &mut clauses)?;
                    enforce_vocabulary(clauses, defs, &formulas)?;
                }
            }
        }
    } else if !dictionary.is_empty() {
        enforce_vocabulary(flat.iter().copied(), &dictionary, &formulas)?;
    }

    Ok(LoweredProgram {
        kb,
        queries,
        range_lookups,
        constraints,
        dictionary,
    })
}

/// Expand `rulebook` blocks into their constituent clause statements, in source
/// order. A rulebook is a *flat* named container, not a separate namespace — its
/// clauses lower into the KB as if written at top level (MYCIN-2026 M2).
///
/// Rulebooks may NOT nest: a `rulebook` directly inside another is a
/// [`LowerError::NestedRulebook`]. This keeps the expansion non-recursive (one
/// level only), so deeply-nested untrusted source cannot drive unbounded
/// recursion here, and it avoids ambiguous nested-`use` scoping (a nested
/// rulebook's own `use` would otherwise be silently dropped). The parser still
/// *parses* nesting; we reject it semantically at this single, well-defined
/// point.
fn flatten_clauses<'a>(
    statements: &'a [Statement],
    out: &mut Vec<&'a Statement>,
) -> Result<(), LowerError> {
    for s in statements {
        match s {
            Statement::Rulebook {
                name: outer,
                statements: inner,
            } => {
                for c in inner {
                    if let Statement::Rulebook {
                        name: inner_name, ..
                    } = c
                    {
                        return Err(LowerError::NestedRulebook {
                            outer: outer.clone(),
                            inner: inner_name.clone(),
                        });
                    }
                    out.push(c);
                }
            }
            other => out.push(other),
        }
    }
    Ok(())
}

/// Does any statement (at top level or inside a rulebook) `use` a dictionary?
/// Selects M2 scoped enforcement over M1 whole-program enforcement. Rulebooks
/// are flat (see [`flatten_clauses`]), so one level of descent suffices.
fn program_contains_use(statements: &[Statement]) -> bool {
    statements.iter().any(|s| match s {
        Statement::Use(_) => true,
        Statement::Rulebook { statements, .. } => {
            statements.iter().any(|i| matches!(i, Statement::Use(_)))
        }
        _ => false,
    })
}

/// The first `use <name>` directly in this statement list (does not descend into
/// rulebooks — a rulebook's own `use` is found by calling this on its body).
fn first_use(statements: &[Statement]) -> Option<&str> {
    statements.iter().find_map(|s| match s {
        Statement::Use(name) => Some(name.as_str()),
        _ => None,
    })
}

/// Verify every hypothesis/finding term used by `statements` is `define`d in
/// `dictionary` and any finding value is in its declared domain. See [`lower`]
/// for the M1 (whole-program) vs M2 (per-`use`-scope) call sites.
fn enforce_vocabulary<'a>(
    statements: impl IntoIterator<Item = &'a Statement>,
    dictionary: &[Define],
    formulas: &HashMap<&str, &FormulaDef>,
) -> Result<(), LowerError> {
    use std::collections::HashMap;
    let dict: HashMap<&str, &DefineKind> = dictionary
        .iter()
        .map(|d| (d.name.as_str(), &d.kind))
        .collect();

    let check_hypothesis = |t: &AstTerm| -> Result<(), LowerError> {
        let (functor, _) = term_functor_value(t);
        match dict.get(functor) {
            Some(DefineKind::Hypothesis) => Ok(()),
            _ => Err(LowerError::UndefinedTerm {
                name: functor.to_string(),
                expected: "hypothesis",
            }),
        }
    };
    let check_finding = |t: &AstTerm| -> Result<(), LowerError> {
        let (functor, value) = term_functor_value(t);
        match dict.get(functor) {
            Some(DefineKind::Finding { values }) => {
                if let Some(v) = value {
                    if !values.iter().any(|d| d == v) {
                        return Err(LowerError::ValueNotInDomain {
                            functor: functor.to_string(),
                            value: v.to_string(),
                            domain: values.clone(),
                        });
                    }
                }
                Ok(())
            }
            _ => Err(LowerError::UndefinedTerm {
                name: functor.to_string(),
                expected: "finding",
            }),
        }
    };

    // A query is valid if it names a defined HYPOTHESIS (the differential query)
    // OR a defined RELATION (a binding query, `? deficient_in(tay_sachs, $E)`).
    // Relational recall is the single-hop special case of the differential, so the
    // vocabulary check accepts either rather than forcing every query to be a
    // hypothesis.
    let check_query = |t: &AstTerm| -> Result<(), LowerError> {
        let (functor, _) = term_functor_value(t);
        // A FORMULA APPLICATION query (`? bmi(body_mass, height)`) names a
        // registered formula, not a hypothesis or relation — accept it here so
        // the closed-vocabulary gate does not reject the very construct this
        // feature introduces. The formula's own parameter-scoping was checked at
        // registration.
        if formulas.contains_key(functor) {
            return Ok(());
        }
        match dict.get(functor) {
            Some(DefineKind::Relation { .. }) => Ok(()),
            _ => check_hypothesis(t),
        }
    };

    for stmt in statements {
        match stmt {
            Statement::Prior { conclusion, .. } => check_hypothesis(conclusion)?,
            Statement::Query { conclusion } => check_query(conclusion)?,
            Statement::Contributes {
                evidence,
                conclusion,
                ..
            } => {
                check_hypothesis(conclusion)?;
                // Only term-evidence is a dictionary finding; a numeric
                // predicate references a valued slot, not a finding.
                if let Evidence::Term(t) = evidence {
                    check_finding(t)?;
                }
            }
            Statement::Interacts {
                evidence_set,
                conclusion,
                ..
            } => {
                check_hypothesis(conclusion)?;
                for t in evidence_set {
                    check_finding(t)?;
                }
            }
            Statement::Uncertain {
                domain, conclusion, ..
            } => {
                check_hypothesis(conclusion)?;
                for t in domain {
                    check_finding(t)?;
                }
            }
            Statement::Observe { term } => check_finding(term)?,
            _ => {}
        }
    }
    Ok(())
}

/// The functor name and (if present) the single atom-argument value of a term:
/// `csf_glucose(low)` → (`csf_glucose`, Some(`low`)); `bacterial` → (`bacterial`,
/// None). A non-atom argument (number / nested) yields no value.
fn term_functor_value(t: &AstTerm) -> (&str, Option<&str>) {
    match t {
        AstTerm::Atom(s) => (s.as_str(), None),
        AstTerm::Compound { functor, args } => {
            let value = match args.first() {
                Some(AstTerm::Atom(v)) => Some(v.as_str()),
                _ => None,
            };
            (functor.as_str(), value)
        }
        AstTerm::Num(_) => ("", None),
        // A variable has no functor name; it never reaches vocabulary
        // enforcement (relate edges and queries with variables are not checked
        // against the finding/hypothesis dictionary).
        AstTerm::Var(_) => ("", None),
    }
}

/// Reject a likelihood ratio that the engine's `from_lr` constructors
/// would panic on (`lr <= 0`) or that is non-finite. Centralised so the
/// `contributes` and `interacts` paths share one guard.
fn check_lr(lr: f64) -> Result<(), LowerError> {
    if lr.is_finite() && lr > 0.0 {
        Ok(())
    } else {
        Err(LowerError::InvalidLikelihoodRatio { value: lr })
    }
}

/// Lower a surface [`NumLit`] to the engine's ground [`CoreNumber`] **without an `f64` hop**
/// (ADJ-EXACT-NUMBERS NX-2): `Int` becomes `Number::Int` (keeping the small-integer fast path),
/// and `Exact` becomes `Number::Exact`, carrying every written digit into the stored value.
fn lower_numlit(n: &NumLit) -> CoreTerm {
    match n {
        NumLit::Int(i) => core_int(*i),
        NumLit::Exact(d) => CoreTerm::Num(CoreNumber::Exact(d.clone())),
    }
}

/// Lower one [`crate::ast::TableCell`] (ADJ-TABLES RS-5) to a ground engine term.
/// The three cell kinds map 1:1 onto the engine's three ground term kinds; a cell
/// is never a variable or compound, so this is total and never fails.
fn lower_cell(cell: &crate::ast::TableCell) -> CoreTerm {
    match cell {
        crate::ast::TableCell::Number(x) => lower_numlit(x),
        crate::ast::TableCell::Atom(name) => core_atom(name),
        crate::ast::TableCell::Text(s) => CoreTerm::Str(s.clone()),
    }
}

fn lower_term(t: &AstTerm) -> CoreTerm {
    match t {
        AstTerm::Atom(name) => core_atom(name),
        AstTerm::Num(x) => lower_numlit(x),
        // A bare `Var` outside a query goal (e.g. inside a `relate` ground edge)
        // is unusual — ground edges have no variables — but lower it to a fresh
        // logic variable rather than panicking, so the engine treats it as an
        // unbound (anonymous) position.
        AstTerm::Var(name) => CoreTerm::Var(core_var(name)),
        AstTerm::Compound { functor, args } => {
            compound(functor, args.iter().map(lower_term).collect())
        }
    }
}

/// Lower a term that may contain logic variables, mapping equal variable *names*
/// to the SAME [`LogicVar`] within one scope (a single query goal). This makes a
/// repeated variable — `same($A, $A)` — unify consistently, the way Prolog
/// variables behave within a clause. Used for query goals; `relate` edges and
/// belief clauses use the var-free [`lower_term`].
fn lower_term_scoped(t: &AstTerm, vars: &mut HashMap<String, LogicVar>) -> CoreTerm {
    match t {
        AstTerm::Atom(name) => core_atom(name),
        AstTerm::Num(x) => lower_numlit(x),
        AstTerm::Var(name) => {
            let lv = vars
                .entry(name.clone())
                .or_insert_with(|| core_var(name))
                .clone();
            CoreTerm::Var(lv)
        }
        AstTerm::Compound { functor, args } => compound(
            functor,
            args.iter().map(|a| lower_term_scoped(a, vars)).collect(),
        ),
    }
}

/// Lower a surface `let` formula to the engine's [`ComputeExpr`].
fn lower_expr(expr: &ExprAst) -> ComputeExpr {
    match expr {
        ExprAst::Ref(slot) => ComputeExpr::Ref(slot.clone()),
        ExprAst::Lit(x) => ComputeExpr::Lit(*x),
        ExprAst::Bin(op, a, b) => ComputeExpr::Bin(
            lower_arith_op(*op),
            Box::new(lower_expr(a)),
            Box::new(lower_expr(b)),
        ),
        ExprAst::Abs(a) => ComputeExpr::Unary(ComputeOp::Abs, Box::new(lower_expr(a))),
        ExprAst::Floor(a) => ComputeExpr::Unary(ComputeOp::Floor, Box::new(lower_expr(a))),
        ExprAst::Ceil(a) => ComputeExpr::Unary(ComputeOp::Ceil, Box::new(lower_expr(a))),
        ExprAst::Round(a) => ComputeExpr::Unary(ComputeOp::Round, Box::new(lower_expr(a))),
        ExprAst::Trunc(a) => ComputeExpr::Unary(ComputeOp::Trunc, Box::new(lower_expr(a))),
        ExprAst::Sign(a) => ComputeExpr::Unary(ComputeOp::Sign, Box::new(lower_expr(a))),
        ExprAst::Call(f, a) => ComputeExpr::Unary(lower_named_fn(*f), Box::new(lower_expr(a))),
        // `round_to(x, n)` (NUM-6a): the precision-carrying narrowing. Lowers to the
        // distinct engine `Round` node (not a unary `ComputeOp`) so the precision `n`
        // and the default half-even mode ride along and the exact-path audit records
        // them (ADJ-NUMERIC-SUBSTRATE §4.1–§4.4). `n` was validated a non-negative
        // integer by the adapter.
        ExprAst::RoundTo(a, spec) => ComputeExpr::Round {
            spec: *spec,
            mode: bignum_core::RoundingMode::HalfEven,
            expr: Box::new(lower_expr(a)),
        },
        // `to_scientific(x, figures)` (NUM-6c): the scientific-notation rendering. Lowers
        // to the distinct engine `ToScientific` node so the significant-figure count and
        // the default half-even mode ride along and the exact-path audit records both the
        // exact source and the rendered string (ADJ-NUMERIC-SUBSTRATE §4.1, §4.3).
        // `figures` was validated (≥ 1, within the cap; default applied) by the adapter.
        ExprAst::ToScientific(a, figures) => ComputeExpr::ToScientific {
            figures: *figures,
            mode: bignum_core::RoundingMode::HalfEven,
            expr: Box::new(lower_expr(a)),
        },
        // `to_percent(x, places)` (NUM-6c): the percentage rendering. Lowers to the
        // distinct engine `ToPercent` node so the decimal-place count and the default
        // half-even mode ride along and the exact-path audit records both the exact
        // source ratio and the rendered `d.dd%` string (ADJ-NUMERIC-SUBSTRATE §4.1, §4.3).
        ExprAst::ToPercent(a, places) => ComputeExpr::ToPercent {
            places: *places,
            mode: bignum_core::RoundingMode::HalfEven,
            expr: Box::new(lower_expr(a)),
        },
        // `to_currency(x, code, places)` (NUM-6c): the money rendering. Lowers to the
        // distinct engine `ToCurrency` node so the currency code, the decimal-place count,
        // and the default half-even mode ride along and the exact-path audit records both
        // the exact source amount and the rendered `CODE d.dd` string.
        ExprAst::ToCurrency(a, code, places) => ComputeExpr::ToCurrency {
            code: code.clone(),
            places: *places,
            mode: bignum_core::RoundingMode::HalfEven,
            expr: Box::new(lower_expr(a)),
        },
        ExprAst::Call2(f, a, b) => ComputeExpr::Bin(
            lower_bin_fn(*f),
            Box::new(lower_expr(a)),
            Box::new(lower_expr(b)),
        ),
        ExprAst::Agg(op, slot) => ComputeExpr::Agg(lower_agg_op(*op), slot.clone()),
        // A formula application (RS-1) is never lowered directly: it is expanded
        // away by [`expand_applies`] BEFORE `lower_expr` runs, so a fully-expanded
        // expression contains no `Apply`. This arm exists only to keep the match
        // total; should a lowering site ever forget to expand, it lowers to a
        // reference on a poisoned slot name so `compute` fails cleanly with
        // `UnknownSlot` rather than silently miscomputing — never a panic, never a
        // wrong number.
        ExprAst::Apply(name, _) => {
            ComputeExpr::Ref(format!("<unexpanded formula application: {name}>"))
        }
    }
}

fn lower_bin_fn(f: BinFn) -> ComputeOp {
    match f {
        BinFn::Min => ComputeOp::Min2,
        BinFn::Max => ComputeOp::Max2,
        BinFn::Gcd => ComputeOp::Gcd,
        BinFn::Lcm => ComputeOp::Lcm,
    }
}

fn lower_named_fn(f: NamedFn) -> ComputeOp {
    match f {
        NamedFn::Sin => ComputeOp::Sin,
        NamedFn::Cos => ComputeOp::Cos,
        NamedFn::Tan => ComputeOp::Tan,
        NamedFn::Ln => ComputeOp::Ln,
        NamedFn::Log => ComputeOp::Log,
        NamedFn::Exp => ComputeOp::Exp,
        NamedFn::Asin => ComputeOp::Asin,
        NamedFn::Acos => ComputeOp::Acos,
        NamedFn::Atan => ComputeOp::Atan,
        NamedFn::Sinh => ComputeOp::Sinh,
        NamedFn::Cosh => ComputeOp::Cosh,
        NamedFn::Tanh => ComputeOp::Tanh,
        NamedFn::Cot => ComputeOp::Cot,
        NamedFn::Sec => ComputeOp::Sec,
        NamedFn::Csc => ComputeOp::Csc,
    }
}

fn lower_arith_op(op: ArithOp) -> ComputeOp {
    match op {
        ArithOp::Add => ComputeOp::Add,
        ArithOp::Sub => ComputeOp::Sub,
        ArithOp::Mul => ComputeOp::Mul,
        ArithOp::Div => ComputeOp::Div,
        ArithOp::Pow => ComputeOp::Pow,
        ArithOp::Mod => ComputeOp::Mod,
    }
}

fn lower_agg_op(op: AggOp) -> ComputeOp {
    match op {
        AggOp::Sum => ComputeOp::Sum,
        AggOp::Count => ComputeOp::Count,
        AggOp::Min => ComputeOp::Min,
        AggOp::Max => ComputeOp::Max,
        AggOp::Avg => ComputeOp::Avg,
    }
}

/// Map the surface comparison operator to the engine's [`EngineCmpOp`].
fn lower_cmp_op(op: CmpOp) -> EngineCmpOp {
    match op {
        CmpOp::Ge => EngineCmpOp::Ge,
        CmpOp::Le => EngineCmpOp::Le,
        CmpOp::Gt => EngineCmpOp::Gt,
        CmpOp::Lt => EngineCmpOp::Lt,
        CmpOp::Eq => EngineCmpOp::Eq,
    }
}

/// Fold a table row's OWN `{ … }` provenance block over the table's envelope
/// (ADJ-TABLES RS-5e), producing the provenance stamped on *that row's* fact.
///
/// ## Why a row needs its own provenance
///
/// A table carries one `source`/`locator`/`trust` envelope. With six bands and one
/// envelope, **every** answer — in every band — quotes the same sentence. That is an
/// accounting error: the audit trail asserts a fact and cites a span that does not
/// defend it. A range lookup makes it glaring (the selected row is explicit in the
/// audit), but it was equally wrong for exact lookup all along.
///
/// ## Override, don't replace
///
/// The row's fields are folded **over** the envelope rather than replacing it, so a
/// row supplies only what differs — usually just the `source` span of its own cell —
/// and inherits the shared `locator` and `trust`. That keeps the common case terse
/// (one `source` per row, one `locator` for the page) and means an empty block is
/// exactly the old behaviour. Corroborating `cites` are *appended* to the envelope's,
/// since they are additional independent support, not a correction.
///
/// Duplicate keys **within one row block** are the same clean error they are anywhere
/// else (`source` twice in one block is a typo, not an override of itself).
fn row_provenance(
    table: &Provenance,
    row_annotations: &[Annotation],
) -> Result<Provenance, LowerError> {
    if row_annotations.is_empty() {
        return Ok(table.clone());
    }
    let mut prov = table.clone();
    let (mut saw_source, mut saw_locator, mut saw_trust) = (false, false, false);
    for a in row_annotations {
        match a {
            Annotation::Source(s) => {
                if saw_source {
                    return Err(LowerError::DuplicateAnnotation { name: "source" });
                }
                saw_source = true;
                prov.source = s.clone();
            }
            Annotation::Locator(s) => {
                if saw_locator {
                    return Err(LowerError::DuplicateAnnotation { name: "locator" });
                }
                saw_locator = true;
                prov.locator = Some(s.clone());
            }
            Annotation::Trust(name) => {
                if saw_trust {
                    return Err(LowerError::DuplicateAnnotation { name: "trust" });
                }
                saw_trust = true;
                prov.trust_tier = match name {
                    TrustTierName::Consensus => TrustTier::Consensus,
                    TrustTierName::Authoritative => TrustTier::Authoritative,
                    TrustTierName::Empirical => TrustTier::Empirical,
                    TrustTierName::Inferred => TrustTier::Inferred,
                    TrustTierName::Unattributed => TrustTier::Unattributed,
                };
            }
            Annotation::Cites { source, locator } => {
                prov.corroborations
                    .push(Citation::new(source.clone(), locator.clone()));
            }
            Annotation::Quote {
                text,
                byte_offset,
                snapshot_hex,
            } => {
                // A row may pin its OWN span (RS-4 PR-D4). Same fail-closed
                // well-formedness checks as the envelope path; a row that does
                // not pin simply inherits whatever the table envelope pinned.
                let hash = ContentHash::from_hex(snapshot_hex).ok_or({
                    LowerError::MalformedQuotePin {
                        reason: "snapshot must be a 64-character lowercase SHA-256 hex string",
                    }
                })?;
                let with = prov
                    .clone()
                    .with_quote(text.clone(), Some(*byte_offset), Some(hash));
                if with.quote.is_unmigrated() {
                    return Err(LowerError::MalformedQuotePin {
                        reason: "quote text has no visible content to anchor",
                    });
                }
                prov = with;
            }
        }
    }
    Ok(prov)
}

fn annotations_to_provenance(annotations: &[Annotation]) -> Result<Provenance, LowerError> {
    let mut source: Option<String> = None;
    let mut locator: Option<String> = None;
    let mut trust: Option<TrustTier> = None;
    // ADJ-A9: corroborating citations are REPEATABLE — accumulate in source
    // order rather than rejecting duplicates.
    let mut corroborations: Vec<Citation> = Vec::new();
    // RS-4 PR-D4: a single optional pinned quote (verbatim text, byte offset,
    // snapshot hash). Applied after the base provenance is built.
    let mut quote_pin: Option<(String, usize, String)> = None;

    for a in annotations {
        match a {
            Annotation::Source(s) => {
                if source.is_some() {
                    return Err(LowerError::DuplicateAnnotation { name: "source" });
                }
                source = Some(s.clone());
            }
            Annotation::Locator(s) => {
                if locator.is_some() {
                    return Err(LowerError::DuplicateAnnotation { name: "locator" });
                }
                locator = Some(s.clone());
            }
            Annotation::Trust(name) => {
                if trust.is_some() {
                    return Err(LowerError::DuplicateAnnotation { name: "trust" });
                }
                trust = Some(match name {
                    TrustTierName::Consensus => TrustTier::Consensus,
                    TrustTierName::Authoritative => TrustTier::Authoritative,
                    TrustTierName::Empirical => TrustTier::Empirical,
                    TrustTierName::Inferred => TrustTier::Inferred,
                    TrustTierName::Unattributed => TrustTier::Unattributed,
                });
            }
            Annotation::Cites { source, locator } => {
                corroborations.push(Citation::new(source.clone(), locator.clone()));
            }
            Annotation::Quote {
                text,
                byte_offset,
                snapshot_hex,
            } => {
                if quote_pin.is_some() {
                    return Err(LowerError::DuplicateAnnotation { name: "quote" });
                }
                quote_pin = Some((text.clone(), *byte_offset, snapshot_hex.clone()));
            }
        }
    }

    // If a source is present but no trust tier was specified, default
    // to Authoritative — the common case for cited rulebooks.
    // Otherwise (no source, no tier), default to Unattributed.
    let trust_tier = trust.unwrap_or_else(|| {
        if source.is_some() {
            TrustTier::Authoritative
        } else {
            TrustTier::Unattributed
        }
    });

    let mut prov = Provenance::new(source.unwrap_or_default(), locator, trust_tier);
    // ADJ-A9: attach any corroborating citations (documentary only — they do
    // not change the LR arithmetic, only what the audit trail can list).
    prov.corroborations = corroborations;

    // RS-4 PR-D4 (§E.3.1): apply a pinned quote, if one was written. The
    // hex must parse to a 64-char SHA-256, and the text must carry visible
    // content — both are FAIL-CLOSED here so a malformed pin never reaches the
    // engine as a half-built `Verbatim` span. The *anchored* check (does the
    // text really sit at `byte_offset` in the snapshot?) runs later, once the
    // program's snapshot bundle is resolvable — a hand-authored offset that
    // doesn't verify is caught there, not silently trusted.
    if let Some((text, byte_offset, snapshot_hex)) = quote_pin {
        let hash = ContentHash::from_hex(&snapshot_hex).ok_or(LowerError::MalformedQuotePin {
            reason: "snapshot must be a 64-character lowercase SHA-256 hex string",
        })?;
        let with = prov.clone().with_quote(text, Some(byte_offset), Some(hash));
        // `with_quote` degrades a blank/invisible span to `Unmigrated`. A pin
        // that names a snapshot but carries no checkable text is malformed, not
        // a silent partial — refuse it.
        if with.quote.is_unmigrated() {
            return Err(LowerError::MalformedQuotePin {
                reason: "quote text has no visible content to anchor",
            });
        }
        prov = with;
    }

    Ok(prov)
}

// ---------------------------------------------------------------------------
// Formula libraries (ADJ-FORMULA-LIBRARIES rung-0)
// ---------------------------------------------------------------------------

/// Validate a `formula` at registration time: (1) **parameter-scoping** — every
/// identifier leaf in the body must name a declared parameter, so a stray
/// identifier is a clean compile error rather than a silent mis-binding at apply
/// time; and (2) the **provenance-required lint** — a shipped formula must carry
/// a non-empty `source`, mirroring the recall-library adversarial write gate.
fn validate_formula(fd: &FormulaDef) -> Result<(), LowerError> {
    // Scope grows LEFT-TO-RIGHT (RS-2): the parameters are in scope everywhere;
    // each `let`-step may reference the parameters plus any EARLIER step's name,
    // and after a step it binds its own name; the final body may reference the
    // parameters plus ALL step names. An identifier that resolves to none of
    // these is a clean free-variable error.
    let mut in_scope: std::collections::HashSet<String> = fd.params.iter().cloned().collect();
    let check =
        |expr: &ExprAst, scope: &std::collections::HashSet<String>| -> Result<(), LowerError> {
            let mut refs = Vec::new();
            collect_refs(expr, &mut refs);
            for r in refs {
                if !scope.contains(&r) {
                    return Err(LowerError::FormulaFreeVariable {
                        formula: fd.name.clone(),
                        variable: r,
                    });
                }
            }
            Ok(())
        };
    for step in &fd.steps {
        check(&step.expr, &in_scope)?;
        in_scope.insert(step.name.clone());
    }
    check(&fd.body, &in_scope)?;
    // Reuse the shared provenance path (the same relate/rule/prior use), then
    // enforce that the primary `source` span is non-empty.
    let prov = annotations_to_provenance(&fd.annotations)?;
    if prov.source.trim().is_empty() {
        return Err(LowerError::FormulaMissingProvenance {
            formula: fd.name.clone(),
        });
    }
    Ok(())
}

/// Collect every identifier leaf of an [`ExprAst`] — a [`ExprAst::Ref`] name or
/// an [`ExprAst::Agg`] slot — into `out`. Used by [`validate_formula`] to check
/// parameter-scoping. Numbers and operators contribute no identifiers.
fn collect_refs(expr: &ExprAst, out: &mut Vec<String>) {
    match expr {
        ExprAst::Ref(name) => out.push(name.clone()),
        ExprAst::Agg(_, slot) => out.push(slot.clone()),
        ExprAst::Lit(_) => {}
        ExprAst::Bin(_, a, b) | ExprAst::Call2(_, a, b) => {
            collect_refs(a, out);
            collect_refs(b, out);
        }
        ExprAst::Abs(a)
        | ExprAst::Floor(a)
        | ExprAst::Ceil(a)
        | ExprAst::Round(a)
        | ExprAst::RoundTo(a, _)
        | ExprAst::ToScientific(a, _)
        | ExprAst::ToPercent(a, _)
        | ExprAst::ToCurrency(a, _, _)
        | ExprAst::Trunc(a)
        | ExprAst::Sign(a)
        | ExprAst::Call(_, a) => collect_refs(a, out),
        // A formula application's callee NAME is a formula reference, not an
        // identifier leaf — so it is NOT a candidate free variable (parameter
        // scoping only constrains the value leaves). Only its ARGUMENTS carry
        // identifier leaves that must resolve to declared parameters. (Whether the
        // callee name resolves to a known formula is checked at APPLY time, not
        // here, so a formula may legally reference a sibling declared later in the
        // same book — forward references resolve in the expansion pass.)
        ExprAst::Apply(_, args) => {
            for a in args {
                collect_refs(a, out);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Formula-application composition (ADJ-RULE-SUBSTRATE RS-1)
// ---------------------------------------------------------------------------

/// Maximum formula-application nesting depth. Mirrors the arithmetic evaluator's
/// own [`logic_engine::compute::MAX_EVAL_DEPTH`] (=256): a genuine composed
/// formula (`ratio = quotient(…)`, `cockcroft_gault = quotient(product(…), …)`)
/// is a handful of applications deep, so this is a **safety backstop** — not a
/// modelling constraint — against a self- or mutually-recursive formula
/// (`formula f(x) = f(x)`) expanding forever. Exceeding it is a clean, typed
/// [`LowerError::FormulaRecursionTooDeep`], never a stack overflow.
const FORMULA_MAX_APPLY_DEPTH: usize = 256;

/// Maximum TOTAL number of AST nodes a single formula application may expand into
/// (ADJ-RULE-SUBSTRATE RS-1). This is the size guard that the depth guard
/// ([`FORMULA_MAX_APPLY_DEPTH`]) cannot subsume: a body that names a parameter
/// more than once (`g(x) = x * x`) duplicates the bound subtree on substitution,
/// so composing such formulas grows the expanded tree exponentially (`2ⁿ`) while
/// depth grows linearly. Charged on every substituted / emitted node and checked
/// BEFORE a duplicated subtree is materialised, so an adversarial formulabook
/// bails in microseconds instead of OOMing. 10_000 nodes is orders of magnitude
/// more than any legitimate composed clinical formula needs (they are a handful of
/// applications deep) yet a negligible fraction of the exponential blow-up.
const FORMULA_MAX_EXPANSION_NODES: usize = 10_000;

/// The largest `n` accepted by the `round_to(x, n)` built-in (NUM-6a). A DoS
/// guard: rounding an exact rational to `n` decimal places materializes up to `n`
/// digits, so an unbounded `n` (`round_to(x, 1000000000)`) would ask the exact
/// path for a gigabyte-scale mantissa. 100 places is far beyond the engine's own
/// default precision (256 bits ≈ 77 significant digits, ADJ-NUMERIC-SUBSTRATE §3)
/// and any real measurement, so the cap constrains only pathological inputs.
const MAX_ROUND_PLACES: u32 = 100;

/// The significant-figure count `to_scientific(x)` uses when the surface omits it
/// (NUM-6c). Six is the conventional default mantissa precision for scientific
/// notation (`6.02214e23`) and comfortably inside [`MAX_ROUND_PLACES`]; an explicit
/// `to_scientific(x, n)` overrides it. The default is applied here at lowering, so the
/// engine node always carries a concrete `figures` count and the audit records what was
/// used regardless of whether the writer stated it.
const DEFAULT_SCI_FIGURES: u32 = 6;

/// The decimal-place count `to_percent(x)` uses when the surface omits it (NUM-6c). Two
/// is the conventional default for a formatted percentage (`"33.33%"`) and matches the
/// precision-sensitive framing of §4 (the `$100M-per-point` case); an explicit
/// `to_percent(x, n)` overrides it. Applied at lowering, so the engine node always carries
/// a concrete `places` count and the audit records what was used. Unlike the significant-
/// figure ops, `places = 0` is a valid explicit argument (`to_percent(x, 0) = "50%"`).
const DEFAULT_PERCENT_PLACES: u32 = 2;

/// The decimal-place count `to_currency(x, code)` uses when the surface omits it (NUM-6c).
/// Two is the common minor-unit precision (cents, pence) and matches the ISO-4217 default
/// for the major currencies; an explicit `to_currency(x, code, n)` overrides it (e.g. `0`
/// for JPY, `3` for KWD). Applied at lowering, so the engine node always carries a concrete
/// `places` count and the audit records what was used. `places = 0` is a valid argument.
const DEFAULT_CURRENCY_PLACES: u32 = 2;

/// Maximum AST-nesting depth the expansion/substitution/clone walkers may descend
/// in a single expression (ADJ-RULE-SUBSTRATE RS-1). The node budget bounds total
/// SIZE but not DEPTH: a left-leaning operator spine (`x + 0 + 0 + … + 0`) is one
/// node wide per level yet arbitrarily deep, and the recursive walkers descend it
/// frame-for-frame — so a deep spine would overflow the native stack (an uncatchable
/// abort) before the node budget, which only trips at the bottom of the descent,
/// could fire. This caps walker recursion so a pathological spine is a clean typed
/// error, never a crash.
///
/// The value (96) matches [`adapter`](crate::adapter)'s `SUBST_DEPTH_BUDGET`, chosen
/// there for the identical reason: these walkers carry non-trivial per-frame state
/// (a `HashMap` of substitutions, `Vec` accumulators), so a *few hundred* debug-build
/// frames already approach the default ~2 MiB worker-thread stack. 96 keeps the walk
/// comfortably within that budget on any stack the compiler might run on, while being
/// far deeper than any legitimate formula body or bound expression (a handful of
/// levels); anything deeper the evaluator's own `MAX_EVAL_DEPTH` would reject anyway.
const FORMULA_MAX_NODE_DEPTH: usize = 96;

/// Descend one AST level, failing with a clean [`LowerError::FormulaNestingTooDeep`]
/// the moment the walker would exceed [`FORMULA_MAX_NODE_DEPTH`] frames. Returns the
/// child depth to pass to the recursive call, so a walker reads
/// `walk(child, …, descend(node_depth)?)` at every recursion — the check happens
/// BEFORE the recursive call is made, so the stack never actually grows past the cap.
fn descend(node_depth: usize) -> Result<usize, LowerError> {
    if node_depth >= FORMULA_MAX_NODE_DEPTH {
        Err(LowerError::FormulaNestingTooDeep {
            limit: FORMULA_MAX_NODE_DEPTH,
        })
    } else {
        Ok(node_depth + 1)
    }
}

/// Charge one node against the expansion budget, failing with a clean
/// [`LowerError::FormulaExpansionTooLarge`] the moment the cap is exceeded. Called
/// on every node [`substitute_expr`] / [`charged_clone`] / [`expand_rec`]
/// materialise, so the `2ⁿ` tree is never fully built — the guard trips partway
/// through the first oversized clone and unwinds.
fn charge(budget: &mut usize) -> Result<(), LowerError> {
    *budget += 1;
    if *budget > FORMULA_MAX_EXPANSION_NODES {
        Err(LowerError::FormulaExpansionTooLarge {
            limit: FORMULA_MAX_EXPANSION_NODES,
        })
    } else {
        Ok(())
    }
}

/// Deep-clone `expr`, charging the expansion [`budget`](charge) for every node.
/// Used by [`substitute_expr`] when a parameter reference is replaced by its bound
/// argument subtree: a naive `.clone()` would copy an arbitrarily large subtree in
/// one unmetered step (the `g(x) = x * x` duplication vector), so cloning THROUGH
/// the budget is what makes the exponential bail early — the clone of the second
/// `x` trips the cap and unwinds before the doubled tree exists.
fn charged_clone(
    expr: &ExprAst,
    budget: &mut usize,
    node_depth: usize,
) -> Result<ExprAst, LowerError> {
    charge(budget)?;
    // Bound the descent as well as the size: a bound argument subtree can itself be
    // an arbitrarily deep operator spine, so cloning it must not recurse without a
    // frame cap (see [`FORMULA_MAX_NODE_DEPTH`]).
    let d = descend(node_depth)?;
    Ok(match expr {
        ExprAst::Ref(_) | ExprAst::Lit(_) | ExprAst::Agg(_, _) => expr.clone(),
        ExprAst::Bin(op, a, b) => ExprAst::Bin(
            *op,
            Box::new(charged_clone(a, budget, d)?),
            Box::new(charged_clone(b, budget, d)?),
        ),
        ExprAst::Call2(f, a, b) => ExprAst::Call2(
            *f,
            Box::new(charged_clone(a, budget, d)?),
            Box::new(charged_clone(b, budget, d)?),
        ),
        ExprAst::Abs(a) => ExprAst::Abs(Box::new(charged_clone(a, budget, d)?)),
        ExprAst::Floor(a) => ExprAst::Floor(Box::new(charged_clone(a, budget, d)?)),
        ExprAst::Ceil(a) => ExprAst::Ceil(Box::new(charged_clone(a, budget, d)?)),
        ExprAst::Round(a) => ExprAst::Round(Box::new(charged_clone(a, budget, d)?)),
        ExprAst::RoundTo(a, n) => ExprAst::RoundTo(Box::new(charged_clone(a, budget, d)?), *n),
        ExprAst::ToScientific(a, n) => {
            ExprAst::ToScientific(Box::new(charged_clone(a, budget, d)?), *n)
        }
        ExprAst::ToPercent(a, n) => ExprAst::ToPercent(Box::new(charged_clone(a, budget, d)?), *n),
        ExprAst::ToCurrency(a, code, n) => {
            ExprAst::ToCurrency(Box::new(charged_clone(a, budget, d)?), code.clone(), *n)
        }
        ExprAst::Trunc(a) => ExprAst::Trunc(Box::new(charged_clone(a, budget, d)?)),
        ExprAst::Sign(a) => ExprAst::Sign(Box::new(charged_clone(a, budget, d)?)),
        ExprAst::Call(f, a) => ExprAst::Call(*f, Box::new(charged_clone(a, budget, d)?)),
        ExprAst::Apply(name, args) => {
            let mut out = Vec::with_capacity(args.len());
            for a in args {
                out.push(charged_clone(a, budget, d)?);
            }
            ExprAst::Apply(name.clone(), out)
        }
    })
}

/// A `contributes … from <formula-app> <op> <thr>` clause held for the deferred
/// branch-on-formula pass (see [`lower`]). Its LHS formula must be computed after
/// every `observe` is in the KB, so the branch (declared in a library) and the
/// numbers (supplied by a consumer) can meet.
struct DeferredFormulaPredicate {
    /// The formula-application LHS (e.g. `bmi(body_mass, height)`).
    lhs: ExprAst,
    op: CmpOp,
    /// The threshold expression (`30`); may itself apply a formula.
    rhs: ExprAst,
    /// The saturating likelihood ratio (already `check_lr`-validated).
    lr: f64,
    /// The verdict this branch supports (`obese`), already lowered.
    conclusion: CoreTerm,
    /// The branch clause's OWN provenance (e.g. WHO's obesity threshold) — distinct
    /// from the applied formula's provenance, which rides on the derived slot.
    prov: Provenance,
}

/// Recursively expand every [`ExprAst::Apply`] in `expr` into the applied
/// formula's substituted body — the RS-1 composition core — returning the
/// fully-expanded, application-free [`ExprAst`] ready for [`lower_expr`], PLUS the
/// ordered provenance chain of every formula applied during expansion.
///
/// ## What "expand" means
///
/// `formula ratio(numerator, denominator) = quotient(numerator, denominator)`
/// applied as `ratio(a, b)` first substitutes `numerator → a`, `denominator → b`
/// to get `quotient(a, b)`; expanding THAT substitutes `quotient`'s own body
/// (`dividend / divisor`) to get `a / b`. Both formulas' provenances are collected
/// so the composed derivation cites each (see [`compose_provenance`]).
///
/// ## Totality — always halt or error, never hang
///
/// `depth` counts formula-body entries (not AST nodes): each time we descend into
/// a callee's substituted body we recurse at `depth + 1`, and once it reaches
/// [`FORMULA_MAX_APPLY_DEPTH`] a self/mutually-recursive formula returns a clean
/// [`LowerError::FormulaRecursionTooDeep`] instead of overflowing the stack. This
/// is the compute analogue of the resolver's recursion guard — the substrate is
/// total by construction.
fn expand_applies(
    expr: &ExprAst,
    formulas: &HashMap<&str, &FormulaDef>,
    depth: usize,
) -> Result<(ExprAst, Vec<Provenance>), LowerError> {
    let mut chain = Vec::new();
    // One shared node budget for the WHOLE expansion. The depth guard bounds
    // recursion LEVELS; this bounds total expanded SIZE, so a body that reuses a
    // parameter (`x * x`) composed n deep cannot balloon to 2^n nodes — the cap
    // trips first, in O(cap) time, with a clean `FormulaExpansionTooLarge`.
    let mut budget: usize = 0;
    // The names of the formulas currently OPEN on the expansion path (a stack). A
    // formula that names itself — directly (`loop(x) = loop(x)`) or through a cycle
    // (`a = b`, `b = a`) — reappears on this path at the moment it re-enters, so we
    // reject it in O(1) at recursion depth 1 rather than letting `expand_rec` nest
    // hundreds of frames deep and overflow the stack before the numeric depth guard
    // trips. This is path-scoped: a formula legitimately applied twice in SIBLING
    // positions (`f(x) = g(x) + g(x)`) is popped off the path between the two uses,
    // so reuse is never mistaken for recursion.
    let mut active: Vec<String> = Vec::new();
    let expanded = expand_rec(
        expr,
        formulas,
        depth,
        &mut chain,
        &mut active,
        &mut budget,
        0,
    )?;
    Ok((expanded, chain))
}

/// The recursive worker for [`expand_applies`]. Walks the expression; at an
/// [`ExprAst::Apply`] it resolves the callee, checks arity, expands the arguments
/// (siblings, same depth), records the callee's provenance, substitutes into the
/// callee body, and expands THAT body at `depth + 1`. Every other node is rebuilt
/// with its children expanded at the same depth.
fn expand_rec(
    expr: &ExprAst,
    formulas: &HashMap<&str, &FormulaDef>,
    depth: usize,
    chain: &mut Vec<Provenance>,
    active: &mut Vec<String>,
    budget: &mut usize,
    node_depth: usize,
) -> Result<ExprAst, LowerError> {
    // Charge one node for every node we visit/emit. This is the SIZE guard: the
    // shared budget is threaded through the entire recursion, so an exponentially
    // branching composition (`pₙ(x) = pₙ₋₁(x) * pₙ₋₁(x)`) trips the cap after
    // `FORMULA_MAX_EXPANSION_NODES` visits — in O(cap) time — instead of building
    // a 2ⁿ tree. The depth guard alone cannot bound this (depth grows linearly
    // while size grows exponentially).
    charge(budget)?;
    // The DEPTH guard, orthogonal to both the size budget and the formula-apply
    // depth: this walker descends one native stack frame per AST level, so a deep
    // operator spine (`x + 0 + 0 + …`) — small in size, huge in depth — would
    // overflow the stack (an uncatchable abort) if only the size budget bounded it,
    // because that budget trips only at the BOTTOM of the descent. `d` is the depth
    // to pass to every child recursion; `descend` errors cleanly the moment the walk
    // would exceed `FORMULA_MAX_NODE_DEPTH` frames (see there).
    let d = descend(node_depth)?;
    match expr {
        ExprAst::Apply(name, args) => {
            // NUM-6a/6b built-ins: `round_to(x, n)` (n decimal PLACES) and
            // `round_sig(x, n)` (n significant FIGURES) — the precision narrowings.
            // Recognised by NAME here, BEFORE the user-formula lookup, so they need no
            // formula definition; they reuse the same comma-list application grammar as
            // user formulas (`quotient(a, b)`), which is why no new grammar or LaTeX
            // surface is required (ADJ-NUMERIC-SUBSTRATE §4.1). The value arg `x` is
            // expanded (it may itself be an application); the precision `n` must be an
            // INTEGER literal within the DoS cap [`MAX_ROUND_PLACES`] — non-negative for
            // `round_to`, and ≥ 1 for `round_sig` (zero significant figures is
            // meaningless). A variable, fraction, out-of-range, or oversized `n` is a
            // clean compile error, never a silent mis-rounding.
            if name == "round_to" || name == "round_sig" {
                if args.len() != 2 {
                    return Err(LowerError::FormulaArity {
                        formula: name.clone(),
                        expected: 2,
                        got: args.len(),
                    });
                }
                let min = if name == "round_sig" { 1.0 } else { 0.0 };
                let n = match &args[1] {
                    ExprAst::Lit(v)
                        if v.fract() == 0.0 && *v >= min && *v <= MAX_ROUND_PLACES as f64 =>
                    {
                        *v as u32
                    }
                    _ => {
                        return Err(LowerError::FormulaBadArgument {
                            formula: name.clone(),
                        })
                    }
                };
                let spec = if name == "round_sig" {
                    logic_engine::RoundSpec::SigFigures(n)
                } else {
                    logic_engine::RoundSpec::Places(n)
                };
                let value = expand_rec(&args[0], formulas, depth, chain, active, budget, d)?;
                return Ok(ExprAst::RoundTo(Box::new(value), spec));
            }
            // NUM-6c built-in: `to_scientific(x [, figures])` — the scientific-notation
            // rendering. Recognised by NAME here, before the user-formula lookup, on the
            // same comma-list application grammar. `figures` is OPTIONAL: `to_scientific(x)`
            // uses the default mantissa precision [`DEFAULT_SCI_FIGURES`]; a stated
            // `to_scientific(x, n)` requires `n` a positive integer literal (`≥ 1`, since a
            // scientific mantissa has at least one significant figure) within the precision
            // cap [`MAX_ROUND_PLACES`]. A non-integer, zero, negative, oversized, or
            // non-literal `n`, or the wrong number of arguments, is a clean compile error.
            if name == "to_scientific" {
                if args.is_empty() || args.len() > 2 {
                    return Err(LowerError::FormulaArity {
                        formula: name.clone(),
                        expected: 2,
                        got: args.len(),
                    });
                }
                let figures = if args.len() == 2 {
                    match &args[1] {
                        ExprAst::Lit(v)
                            if v.fract() == 0.0 && *v >= 1.0 && *v <= MAX_ROUND_PLACES as f64 =>
                        {
                            *v as u32
                        }
                        _ => {
                            return Err(LowerError::FormulaBadArgument {
                                formula: name.clone(),
                            })
                        }
                    }
                } else {
                    DEFAULT_SCI_FIGURES
                };
                let value = expand_rec(&args[0], formulas, depth, chain, active, budget, d)?;
                return Ok(ExprAst::ToScientific(Box::new(value), figures));
            }
            // NUM-6c built-in: `to_percent(x [, places])` — the percentage rendering.
            // Recognised by NAME here, before the user-formula lookup, on the same
            // comma-list application grammar. `places` is OPTIONAL: `to_percent(x)` uses
            // the default [`DEFAULT_PERCENT_PLACES`]; a stated `to_percent(x, n)` requires
            // `n` a NON-NEGATIVE integer literal (`≥ 0` — zero places is meaningful,
            // `"50%"`) within the precision cap [`MAX_ROUND_PLACES`]. A non-integer,
            // negative, oversized, or non-literal `n`, or the wrong argument count, is a
            // clean compile error.
            if name == "to_percent" {
                if args.is_empty() || args.len() > 2 {
                    return Err(LowerError::FormulaArity {
                        formula: name.clone(),
                        expected: 2,
                        got: args.len(),
                    });
                }
                let places = if args.len() == 2 {
                    match &args[1] {
                        ExprAst::Lit(v)
                            if v.fract() == 0.0 && *v >= 0.0 && *v <= MAX_ROUND_PLACES as f64 =>
                        {
                            *v as u32
                        }
                        _ => {
                            return Err(LowerError::FormulaBadArgument {
                                formula: name.clone(),
                            })
                        }
                    }
                } else {
                    DEFAULT_PERCENT_PLACES
                };
                let value = expand_rec(&args[0], formulas, depth, chain, active, budget, d)?;
                return Ok(ExprAst::ToPercent(Box::new(value), places));
            }
            // NUM-6c built-in: `to_currency(x, code [, places])` — the money rendering.
            // Recognised by NAME here, before the user-formula lookup. The SECOND argument
            // is the currency CODE — a bare identifier (`USD`) taken verbatim: it parses as
            // an `ExprAst::Ref` and is NOT resolved as a slot (we read its name directly and
            // never expand it). `places` is OPTIONAL (3rd arg): default [`DEFAULT_CURRENCY_PLACES`];
            // a stated count must be a NON-NEGATIVE integer literal (`≥ 0` — `to_currency(x,
            // JPY, 0)`) within the precision cap [`MAX_ROUND_PLACES`]. A missing/non-identifier
            // code, a non-integer/negative/oversized/non-literal `places`, or the wrong argument
            // count is a clean compile error.
            if name == "to_currency" {
                if args.len() < 2 || args.len() > 3 {
                    return Err(LowerError::FormulaArity {
                        formula: name.clone(),
                        expected: 3,
                        got: args.len(),
                    });
                }
                // The code is a bare identifier (an un-expanded `Ref`); anything else is a
                // bad argument (a number, a formula application, an already-substituted value).
                // Identifiers lex lowercase (`usd`), but currency codes are canonically the
                // uppercase ISO-4217 form, so we normalize to uppercase for the rendered
                // `CODE d.dd` string and the audit record — `usd` → `USD 33.33`.
                let code = match &args[1] {
                    ExprAst::Ref(c) if !c.is_empty() => c.to_uppercase(),
                    _ => {
                        return Err(LowerError::FormulaBadArgument {
                            formula: name.clone(),
                        })
                    }
                };
                let places = if args.len() == 3 {
                    match &args[2] {
                        ExprAst::Lit(v)
                            if v.fract() == 0.0 && *v >= 0.0 && *v <= MAX_ROUND_PLACES as f64 =>
                        {
                            *v as u32
                        }
                        _ => {
                            return Err(LowerError::FormulaBadArgument {
                                formula: name.clone(),
                            })
                        }
                    }
                } else {
                    DEFAULT_CURRENCY_PLACES
                };
                let value = expand_rec(&args[0], formulas, depth, chain, active, budget, d)?;
                return Ok(ExprAst::ToCurrency(Box::new(value), code, places));
            }
            // Resolve the callee against the SAME registry the top-level query path
            // uses. An unknown name is a clean, specific error — distinct from an
            // aggregation or built-in call, which never reach here (they are separate
            // AST nodes recognised earlier in the grammar).
            let fd = *formulas
                .get(name.as_str())
                .ok_or_else(|| LowerError::FormulaUnknown { name: name.clone() })?;
            if fd.params.len() != args.len() {
                return Err(LowerError::FormulaArity {
                    formula: name.clone(),
                    expected: fd.params.len(),
                    got: args.len(),
                });
            }
            // Expand the ARGUMENTS first — an argument may itself be an application
            // (`ratio(product(a, b), c)`). They are siblings of this call, so they
            // expand at the SAME depth (they are not a deeper formula body).
            let mut subst: HashMap<String, ExprAst> = HashMap::new();
            for (param, arg) in fd.params.iter().zip(args.iter()) {
                subst.insert(
                    param.clone(),
                    expand_rec(arg, formulas, depth, chain, active, budget, d)?,
                );
            }
            // The cycle guard: if this formula is already OPEN on the expansion path,
            // re-entering it is (directly or mutually) recursive. ADJ formulas have
            // no base case, so a cycle can only diverge — reject it here, in O(1), at
            // the moment of re-entry, so we never nest deep enough to overflow the
            // stack.
            if active.iter().any(|n| n == name) {
                return Err(LowerError::FormulaRecursionTooDeep {
                    formula: name.clone(),
                    limit: FORMULA_MAX_APPLY_DEPTH,
                });
            }
            // The depth guard: a belt-and-suspenders bound on a legitimately deep (but
            // acyclic) composition. Entering the callee's body is one level deeper.
            if depth + 1 >= FORMULA_MAX_APPLY_DEPTH {
                return Err(LowerError::FormulaRecursionTooDeep {
                    formula: name.clone(),
                    limit: FORMULA_MAX_APPLY_DEPTH,
                });
            }
            // Record this formula's provenance in the composition chain (outer
            // formulas appear before the inner formulas they call).
            chain.push(annotations_to_provenance(&fd.annotations)?);
            // Bind params → expanded args, substitute into the callee body (charged
            // against the same budget), and expand that body in turn
            // (formula-calls-formula) at depth + 1. The callee is pushed onto the
            // active path for the duration of its body expansion and popped after, so
            // a later SIBLING application of the same formula is not seen as a cycle.
            // Fold the callee's `let`-steps (RS-2) into its effective body FIRST
            // so a multi-step callee composes correctly when called from another
            // formula, then substitute the caller's args into it.
            let callee_body = effective_body(fd, budget, d)?;
            let substituted = substitute_expr(&callee_body, &subst, budget, d)?;
            active.push(name.clone());
            let expanded = expand_rec(&substituted, formulas, depth + 1, chain, active, budget, d);
            active.pop();
            expanded
        }
        // Leaves carry no application.
        ExprAst::Ref(_) | ExprAst::Lit(_) | ExprAst::Agg(_, _) => Ok(expr.clone()),
        // Binary nodes: expand both operands at the same depth.
        ExprAst::Bin(op, a, b) => Ok(ExprAst::Bin(
            *op,
            Box::new(expand_rec(a, formulas, depth, chain, active, budget, d)?),
            Box::new(expand_rec(b, formulas, depth, chain, active, budget, d)?),
        )),
        ExprAst::Call2(f, a, b) => Ok(ExprAst::Call2(
            *f,
            Box::new(expand_rec(a, formulas, depth, chain, active, budget, d)?),
            Box::new(expand_rec(b, formulas, depth, chain, active, budget, d)?),
        )),
        // Unary nodes: expand the single operand.
        ExprAst::Abs(a) => Ok(ExprAst::Abs(Box::new(expand_rec(
            a, formulas, depth, chain, active, budget, d,
        )?))),
        ExprAst::Floor(a) => Ok(ExprAst::Floor(Box::new(expand_rec(
            a, formulas, depth, chain, active, budget, d,
        )?))),
        ExprAst::Ceil(a) => Ok(ExprAst::Ceil(Box::new(expand_rec(
            a, formulas, depth, chain, active, budget, d,
        )?))),
        ExprAst::Round(a) => Ok(ExprAst::Round(Box::new(expand_rec(
            a, formulas, depth, chain, active, budget, d,
        )?))),
        ExprAst::RoundTo(a, n) => Ok(ExprAst::RoundTo(
            Box::new(expand_rec(a, formulas, depth, chain, active, budget, d)?),
            *n,
        )),
        ExprAst::ToScientific(a, n) => Ok(ExprAst::ToScientific(
            Box::new(expand_rec(a, formulas, depth, chain, active, budget, d)?),
            *n,
        )),
        ExprAst::ToPercent(a, n) => Ok(ExprAst::ToPercent(
            Box::new(expand_rec(a, formulas, depth, chain, active, budget, d)?),
            *n,
        )),
        ExprAst::ToCurrency(a, code, n) => Ok(ExprAst::ToCurrency(
            Box::new(expand_rec(a, formulas, depth, chain, active, budget, d)?),
            code.clone(),
            *n,
        )),
        ExprAst::Trunc(a) => Ok(ExprAst::Trunc(Box::new(expand_rec(
            a, formulas, depth, chain, active, budget, d,
        )?))),
        ExprAst::Sign(a) => Ok(ExprAst::Sign(Box::new(expand_rec(
            a, formulas, depth, chain, active, budget, d,
        )?))),
        ExprAst::Call(f, a) => Ok(ExprAst::Call(
            *f,
            Box::new(expand_rec(a, formulas, depth, chain, active, budget, d)?),
        )),
    }
}

/// Compose a derived value's provenance from a `primary` envelope plus the chain
/// of formulas applied to produce it (RS-1). The primary is the outer claim's own
/// cite (e.g. `ratio`'s definition); each applied formula's primary span becomes a
/// **corroborating** [`Citation`] (documentary only — it carries no LR weight, it
/// just lets the audit trail list every formula that participated). So a value
/// computed as `ratio` *via* `quotient` renders BOTH citations.
fn compose_provenance(mut primary: Provenance, chain: &[Provenance]) -> Provenance {
    for p in chain {
        if !p.source.trim().is_empty() {
            primary.corroborations.push(Citation::new(
                p.source.clone(),
                p.locator.clone().unwrap_or_default(),
            ));
        }
        // Preserve any corroborations the applied formula itself carried.
        for c in &p.corroborations {
            primary.corroborations.push(c.clone());
        }
    }
    primary
}

/// Compose provenance for a derivation that has NO claim of its own (a `let`, or a
/// branch-on-formula derived slot) but applied one or more formulas. The first
/// applied formula is the primary; the rest corroborate. Returns `None` when the
/// chain is empty (a plain `let` over observed slots — no library claim to cite).
fn compose_chain(chain: &[Provenance]) -> Option<Provenance> {
    let (first, rest) = chain.split_first()?;
    Some(compose_provenance(first.clone(), rest))
}

/// If the query `goal` is a FORMULA APPLICATION — its functor names a registered
/// formula and its argument count matches that formula's parameter count —
/// return the matching definition. A ground atom, a `$variable` goal, or an
/// arity mismatch returns `None` (it is an ordinary query, not an application).
fn formula_for_query<'a>(
    goal: &AstTerm,
    formulas: &HashMap<&str, &'a FormulaDef>,
) -> Option<&'a FormulaDef> {
    if let AstTerm::Compound { functor, args } = goal {
        if let Some(fd) = formulas.get(functor.as_str()) {
            if fd.params.len() == args.len() {
                return Some(fd);
            }
        }
    }
    None
}

/// APPLY a formula: bind each parameter to the correspondingly-positioned
/// argument and substitute into the formula body, yielding a parameter-free
/// [`ExprAst`] ready for the existing [`lower_expr`]/`compute` path. An argument
/// that is a plain identifier binds the parameter to a like-named slot
/// (`ExprAst::Ref`); a number literal binds it to that constant (`ExprAst::Lit`).
/// Any other argument shape is a [`LowerError::FormulaBadArgument`].
fn apply_formula(fd: &FormulaDef, goal: &AstTerm) -> Result<ExprAst, LowerError> {
    let args: &[AstTerm] = match goal {
        AstTerm::Compound { args, .. } => args,
        _ => &[],
    };
    let mut subst: HashMap<String, ExprAst> = HashMap::new();
    for (param, arg) in fd.params.iter().zip(args.iter()) {
        let bound = match arg {
            AstTerm::Atom(name) => ExprAst::Ref(name.clone()),
            // A numeric formula argument feeds the (inherently `f64`) compute layer, so it
            // takes the labeled-lossy export here — the exact literal is preserved on the
            // ground-term paths (`lower_numlit`), not on this compute leaf.
            AstTerm::Num(x) => ExprAst::Lit(x.to_f64_lossy()),
            _ => {
                return Err(LowerError::FormulaBadArgument {
                    formula: fd.name.clone(),
                })
            }
        };
        subst.insert(param.clone(), bound);
    }
    // A shallow, one-level substitution of leaf bindings (a query's args are atoms
    // or numbers) — its own local budget guards a body that reuses a parameter; the
    // deeper formula-calls-formula expansion of the resulting body runs later, in
    // `expand_applies`, under that call's own shared budget. The `let`-steps
    // (RS-2) are folded into a single effective body FIRST (so the param
    // substitution below reaches into the inlined steps), then param-substituted.
    let mut budget: usize = 0;
    let effective = effective_body(fd, &mut budget, 0)?;
    substitute_expr(&effective, &subst, &mut budget, 0)
}

/// Fold a formula's multi-step `let`-bindings (RS-2) into a SINGLE effective
/// body by in-order substitution: each step's expression is expanded against
/// the already-expanded earlier steps, then the final body is expanded against
/// all of them. The result names only the formula's parameters (plus numbers
/// and nested formula applications), so the RS-1 param-substitution and
/// formula-calls-formula expansion consume it unchanged — a multi-step body is
/// surface sugar for the equivalent nested single expression. A formula with no
/// steps returns its body verbatim. Every emitted node is charged against the
/// shared `budget`, so an adversarial step chain (each step doubling the last)
/// trips [`LowerError::FormulaExpansionTooLarge`] instead of exploding.
fn effective_body(
    fd: &FormulaDef,
    budget: &mut usize,
    node_depth: usize,
) -> Result<ExprAst, LowerError> {
    if fd.steps.is_empty() {
        return Ok(fd.body.clone());
    }
    let mut step_subst: HashMap<String, ExprAst> = HashMap::new();
    for step in &fd.steps {
        // Expand this step against the already-inlined earlier steps, then bind
        // its (now step-free) value under its name for the steps/body that follow.
        let expanded = substitute_expr(&step.expr, &step_subst, budget, node_depth)?;
        step_subst.insert(step.name.clone(), expanded);
    }
    substitute_expr(&fd.body, &step_subst, budget, node_depth)
}

/// Substitute parameter references in a formula body with their bound argument
/// expressions. A [`ExprAst::Ref`] naming a parameter becomes the bound
/// expression; a non-parameter identifier is left as-is (validation already
/// proved every identifier is a parameter, so this branch is defensive). An
/// [`ExprAst::Agg`] slot naming a parameter is rewritten to the bound slot name
/// when the binding is itself a slot reference (an aggregation folds a named
/// slot, so only a `Ref` binding is meaningful there).
fn substitute_expr(
    expr: &ExprAst,
    subst: &HashMap<String, ExprAst>,
    budget: &mut usize,
    node_depth: usize,
) -> Result<ExprAst, LowerError> {
    // Charge for the node we are about to emit. Every substituted node counts
    // against the shared expansion budget, so a body that duplicates a bound
    // subtree (`x * x`) can only do so until the cap trips — the exponential tree
    // is never fully materialised.
    charge(budget)?;
    // Bound the descent too: a formula body (or a bound argument) may be a deep
    // operator spine, and the size budget alone only trips at the BOTTOM of the
    // descent — too late to prevent a stack overflow (see [`FORMULA_MAX_NODE_DEPTH`]).
    let d = descend(node_depth)?;
    Ok(match expr {
        // A parameter reference expands to its bound argument subtree — cloned
        // THROUGH the budget so a large binding cannot be duplicated for free. This
        // is the guarded version of the old `.clone()`; the second `x` in `x * x`
        // is what trips the cap on an adversarial composition.
        ExprAst::Ref(name) => match subst.get(name) {
            Some(bound) => charged_clone(bound, budget, d)?,
            None => expr.clone(),
        },
        ExprAst::Lit(_) => expr.clone(),
        ExprAst::Agg(op, slot) => match subst.get(slot) {
            Some(ExprAst::Ref(bound)) => ExprAst::Agg(*op, bound.clone()),
            _ => expr.clone(),
        },
        ExprAst::Bin(op, a, b) => ExprAst::Bin(
            *op,
            Box::new(substitute_expr(a, subst, budget, d)?),
            Box::new(substitute_expr(b, subst, budget, d)?),
        ),
        ExprAst::Call2(f, a, b) => ExprAst::Call2(
            *f,
            Box::new(substitute_expr(a, subst, budget, d)?),
            Box::new(substitute_expr(b, subst, budget, d)?),
        ),
        ExprAst::Abs(a) => ExprAst::Abs(Box::new(substitute_expr(a, subst, budget, d)?)),
        ExprAst::Floor(a) => ExprAst::Floor(Box::new(substitute_expr(a, subst, budget, d)?)),
        ExprAst::Ceil(a) => ExprAst::Ceil(Box::new(substitute_expr(a, subst, budget, d)?)),
        ExprAst::Round(a) => ExprAst::Round(Box::new(substitute_expr(a, subst, budget, d)?)),
        ExprAst::RoundTo(a, n) => {
            ExprAst::RoundTo(Box::new(substitute_expr(a, subst, budget, d)?), *n)
        }
        ExprAst::ToScientific(a, n) => {
            ExprAst::ToScientific(Box::new(substitute_expr(a, subst, budget, d)?), *n)
        }
        ExprAst::ToPercent(a, n) => {
            ExprAst::ToPercent(Box::new(substitute_expr(a, subst, budget, d)?), *n)
        }
        ExprAst::ToCurrency(a, code, n) => ExprAst::ToCurrency(
            Box::new(substitute_expr(a, subst, budget, d)?),
            code.clone(),
            *n,
        ),
        ExprAst::Trunc(a) => ExprAst::Trunc(Box::new(substitute_expr(a, subst, budget, d)?)),
        ExprAst::Sign(a) => ExprAst::Sign(Box::new(substitute_expr(a, subst, budget, d)?)),
        ExprAst::Call(f, a) => ExprAst::Call(*f, Box::new(substitute_expr(a, subst, budget, d)?)),
        // Substitute into a formula application's ARGUMENTS, preserving the callee
        // name. This is what makes `formula ratio(numerator, denominator) =
        // quotient(numerator, denominator)` compose: applying `ratio(a, b)`
        // substitutes `numerator → a`, `denominator → b` INSIDE the nested
        // `quotient(numerator, denominator)`, yielding `quotient(a, b)` — which the
        // expansion pass then resolves in turn.
        ExprAst::Apply(name, args) => {
            let mut out = Vec::with_capacity(args.len());
            for a in args {
                out.push(substitute_expr(a, subst, budget, d)?);
            }
            ExprAst::Apply(name.clone(), out)
        }
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile;
    use logic_engine::{enumerate_all, search, SearchMode, SearchResult};
    use std::str::FromStr;

    // ---- ADJ-FORMULA-LIBRARIES rung-0: formulabook / formula ----

    #[test]
    fn formula_applies_and_carries_its_cited_provenance() {
        // A single-file program that DEFINES a provenanced formula, binds its
        // variables from `observe`d facts, and APPLIES it — the whole rung-0 loop.
        // The engine computes 70 / 1.75² = 22.857 on the CPU, and the derived value
        // carries the formula's WHO citation (source non-empty, trust authoritative).
        let src = r#"
            dictionary bmi_vocab {
                define body_mass : finding
                define height    : finding
                define bmi       : finding
            }
            formulabook body_metrics {
                use bmi_vocab
                formula bmi(body_mass, height) = body_mass / (height * height)
                    source "BMI is weight in kilograms divided by the square of height in metres."
                    locator "https://www.who.int/health-topics/obesity"
                    trust authoritative
            }
            observe body_mass(70)
            observe height(1.75)
            ? bmi(body_mass, height)
        "#;
        let lowered = compile(src).unwrap();
        // A formula application is a derived value, not a differential query.
        assert!(
            lowered.queries.is_empty(),
            "formula query is not a hypothesis"
        );
        let d = lowered
            .kb
            .derived_for("bmi")
            .expect("the formula bound a derived `bmi`");
        assert!(
            (d.value - 22.857).abs() < 0.01,
            "expected ≈22.857, got {}",
            d.value
        );
        let prov = d
            .provenance
            .as_ref()
            .expect("derivation carries provenance");
        assert!(!prov.source.is_empty(), "the WHO source span is attached");
        assert_eq!(prov.trust_tier, TrustTier::Authoritative);
        assert_eq!(
            prov.locator.as_deref(),
            Some("https://www.who.int/health-topics/obesity")
        );
    }

    #[test]
    fn formula_rebinds_to_differently_named_arguments() {
        // The parameters are FORMAL: applying `bmi(weight, stature)` substitutes
        // param→argument, so the engine reads the `weight`/`stature` slots even
        // though the formula was written over `body_mass`/`height`.
        let src = r#"
            formulabook m {
                formula bmi(body_mass, height) = body_mass / (height * height)
                    source "WHO BMI definition." trust authoritative
            }
            observe weight(80)
            observe stature(2)
            ? bmi(weight, stature)
        "#;
        let lowered = compile(src).unwrap();
        let d = lowered.kb.derived_for("bmi").unwrap();
        assert!(
            (d.value - 20.0).abs() < 1e-9,
            "80 / 2² = 20, got {}",
            d.value
        );
    }

    #[test]
    fn formula_free_variable_is_a_clean_scoping_error() {
        // `b` is not a declared parameter — a stray identifier must be rejected,
        // never silently mis-bound to some observed slot at apply time.
        let src = r#"
            formulabook m {
                formula f(a) = a + b
                    source "x" trust authoritative
            }
        "#;
        let err = compile(src).unwrap_err();
        assert!(
            matches!(
                err,
                crate::CompileError::Lower(LowerError::FormulaFreeVariable { ref variable, .. })
                    if variable == "b"
            ),
            "{err:?}"
        );
    }

    #[test]
    fn shipped_formula_without_provenance_is_rejected() {
        // The provenance-required lint: a formula is a claim about the world and
        // may not enter a library unsourced.
        let src = r#"
            formulabook m {
                formula f(a) = a + a
            }
        "#;
        let err = compile(src).unwrap_err();
        assert!(
            matches!(
                err,
                crate::CompileError::Lower(LowerError::FormulaMissingProvenance { ref formula })
                    if formula == "f"
            ),
            "{err:?}"
        );
    }

    #[test]
    fn three_parameter_formula_applies() {
        // Arity > 2: a mean-arterial-pressure-shaped 3-parameter formula. Confirms
        // parameters bind positionally and the body substitutes all three.
        let src = r#"
            formulabook hemo {
                formula weighted3(a, b, c) = (a + b + c) / 3
                    source "arithmetic mean of three readings" trust authoritative
            }
            observe a(3)
            observe b(6)
            observe c(9)
            ? weighted3(a, b, c)
        "#;
        let lowered = compile(src).unwrap();
        let d = lowered.kb.derived_for("weighted3").unwrap();
        assert!(
            (d.value - 6.0).abs() < 1e-9,
            "(3+6+9)/3 = 6, got {}",
            d.value
        );
    }

    #[test]
    fn formulabook_round_trips_through_the_parser() {
        // Round-trip render: the surface `formulabook` parses to exactly the
        // expected typed AST — name, parameters, body shape, and the provenance
        // annotations — so a downstream consumer (formatter, doc-gen) can rebuild it.
        let program = crate::parse(
            "formulabook m {\n\
               use v\n\
               formula bmi(body_mass, height) = body_mass / (height * height)\n\
                 source \"WHO\" locator \"loc\" trust authoritative\n\
             }\n",
        )
        .unwrap();
        let fb = program
            .statements
            .iter()
            .find(|s| matches!(s, Statement::Formulabook { .. }))
            .expect("a formulabook statement");
        let Statement::Formulabook {
            name,
            uses,
            formulas,
        } = fb
        else {
            unreachable!()
        };
        assert_eq!(name, "m");
        assert_eq!(uses, &vec!["v".to_string()]);
        assert_eq!(formulas.len(), 1);
        let f = &formulas[0];
        assert_eq!(f.name, "bmi");
        assert_eq!(
            f.params,
            vec!["body_mass".to_string(), "height".to_string()]
        );
        // body = body_mass / (height * height)
        assert_eq!(
            f.body,
            ExprAst::Bin(
                ArithOp::Div,
                Box::new(ExprAst::Ref("body_mass".into())),
                Box::new(ExprAst::Bin(
                    ArithOp::Mul,
                    Box::new(ExprAst::Ref("height".into())),
                    Box::new(ExprAst::Ref("height".into())),
                )),
            )
        );
        assert_eq!(
            f.annotations,
            vec![
                Annotation::Source("WHO".into()),
                Annotation::Locator("loc".into()),
                Annotation::Trust(TrustTierName::Authoritative),
            ]
        );
    }

    #[test]
    fn formula_end_to_end_via_import_of_the_shipped_library() {
        // The real consumer surface: `import "bmi.adj"` + bind + apply. Uses the
        // SHIPPED library file (loaded from disk) through an in-memory provider, so
        // this test also proves `code/specs/data/adj-formula-stdlib/clinical/bmi.adj`
        // parses, lints (provenance-required), and computes.
        use crate::{compile_with_imports, ImportLimits, ImportProvider};
        use std::collections::HashMap;

        struct Mem {
            files: HashMap<String, String>,
        }
        impl ImportProvider for Mem {
            fn resolve(&self, _importer: &str, literal: &str) -> Result<String, String> {
                self.files
                    .contains_key(literal)
                    .then(|| literal.to_string())
                    .ok_or_else(|| format!("no such file: {literal}"))
            }
            fn load(&self, canonical: &str) -> Result<String, String> {
                self.files
                    .get(canonical)
                    .cloned()
                    .ok_or_else(|| format!("no such file: {canonical}"))
            }
        }

        let bmi_lib = include_str!("../../../../specs/data/adj-formula-stdlib/clinical/bmi.adj");
        let consumer = "import \"bmi.adj\"\n\
                        observe body_mass(70)\n\
                        observe height(1.75)\n\
                        ? bmi(body_mass, height)\n";
        let mut files = HashMap::new();
        files.insert("bmi.adj".to_string(), bmi_lib.to_string());
        files.insert("consumer.adj".to_string(), consumer.to_string());
        let provider = Mem { files };

        let lowered =
            compile_with_imports("consumer.adj", &provider, ImportLimits::default()).unwrap();
        let d = lowered
            .kb
            .derived_for("bmi")
            .expect("applied imported formula");
        assert!(
            (d.value - 22.857).abs() < 0.01,
            "expected ≈22.857, got {}",
            d.value
        );
        let prov = d.provenance.as_ref().expect("carries the library citation");
        assert!(
            prov.source.contains("weight") || prov.source.contains("BMI"),
            "WHO source span attached: {}",
            prov.source
        );
        assert_eq!(prov.trust_tier, TrustTier::Authoritative);
    }

    // ---- REL-2: relational recall (relate edges + binding queries) ----

    /// Pull the single logic variable out of a lowered query goal, so a test can
    /// read its binding from a proof's substitution.
    fn query_var(q: &CoreTerm) -> LogicVar {
        match q {
            CoreTerm::Compound { args, .. } => args
                .iter()
                .find_map(|a| match a {
                    CoreTerm::Var(v) => Some(v.clone()),
                    _ => None,
                })
                .expect("query goal has a variable"),
            _ => panic!("expected a compound query goal"),
        }
    }

    #[test]
    fn relate_edge_lowers_to_a_provenanced_fact() {
        let src = r#"
            relate deficient_in(tay_sachs, hexosaminidase_a)
                source "Tay-Sachs results from deficient hexosaminidase A."
                trust authoritative
        "#;
        let lowered = compile(src).unwrap();
        // The edge became a Fact carrying its citation as provenance.
        let dag = enumerate_all(
            &compound(
                "deficient_in",
                vec![core_atom("tay_sachs"), core_atom("hexosaminidase_a")],
            ),
            &lowered.kb,
        );
        assert_eq!(dag.proofs.len(), 1, "the ground edge is a fact in the KB");
    }

    #[test]
    fn forward_recall_binds_the_enzyme_with_a_proof() {
        // "Which enzyme is deficient in Tay-Sachs?" — the single-hop binding query.
        let src = r#"
            relate deficient_in(tay_sachs, hexosaminidase_a) trust authoritative
            relate deficient_in(gaucher, glucocerebrosidase) trust authoritative
            ? deficient_in(tay_sachs, $Enzyme)
        "#;
        let lowered = compile(src).unwrap();
        let query = &lowered.queries[0];
        let v = query_var(query);
        let dag = enumerate_all(query, &lowered.kb);
        assert_eq!(
            dag.proofs.len(),
            1,
            "exactly one enzyme is deficient in Tay-Sachs"
        );
        assert_eq!(
            dag.proofs[0].bindings.walk_var(&v),
            core_atom("hexosaminidase_a")
        );
    }

    #[test]
    fn reverse_lookup_is_free() {
        // "Which disease lacks hexosaminidase A?" — variable on the other side.
        let src = r#"
            relate deficient_in(tay_sachs, hexosaminidase_a) trust authoritative
            relate deficient_in(gaucher, glucocerebrosidase) trust authoritative
            ? deficient_in($Disease, hexosaminidase_a)
        "#;
        let lowered = compile(src).unwrap();
        let query = &lowered.queries[0];
        let v = query_var(query);
        let dag = enumerate_all(query, &lowered.kb);
        assert_eq!(dag.proofs.len(), 1);
        assert_eq!(dag.proofs[0].bindings.walk_var(&v), core_atom("tay_sachs"));
    }

    #[test]
    fn recall_abstains_on_an_ungrounded_disease() {
        // No grounded edge → no proofs → UNKNOWN (the honest failure mode).
        let src = r#"
            relate deficient_in(tay_sachs, hexosaminidase_a) trust authoritative
            ? deficient_in(niemann_pick, $Enzyme)
        "#;
        let lowered = compile(src).unwrap();
        let dag = enumerate_all(&lowered.queries[0], &lowered.kb);
        assert!(
            dag.proofs.is_empty(),
            "must abstain, not fabricate an enzyme"
        );
    }

    // ---- ADJ-TABLES RS-5: the `table` construct ----

    #[test]
    fn table_rows_lower_to_provenanced_relations() {
        // Each `row` becomes a ground relation `length_to_metres(unit, metres)`
        // carrying the table's provenance — so an exact lookup is the ordinary
        // binding query, and the answer names the table's citation as its proof.
        let src = r#"
            table length_to_metres {
                columns unit, metres
                row (foot, 0.3048)
                row (mile, 1609.344)
                source "Defined with respect to meter"
                locator "https://example.test/nist"
                trust authoritative
            }
            ? length_to_metres(foot, $Metres)
        "#;
        let lowered = compile(src).unwrap();
        // A table is data, not a hypothesis differential.
        let query = &lowered.queries[0];
        let v = query_var(query);
        let dag = enumerate_all(query, &lowered.kb);
        assert_eq!(
            dag.proofs.len(),
            1,
            "exactly one row matches the key `foot`"
        );
        // NX-2: a fractional cell binds as an EXACT decimal, not a lossy `f64` (a distinct
        // `Number` variant). Preserving the exact value is the whole intent — `0.3048` is stored
        // with every digit, and only the *rendering* falls back to the f64 form when it fits.
        assert_eq!(
            dag.proofs[0].bindings.walk_var(&v),
            CoreTerm::Num(CoreNumber::Exact(
                bignum_core::BigDecimal::from_str("0.3048").unwrap()
            ))
        );
        // The answer's proof is a table row whose Fact carries the citation.
        let fid = dag.proofs[0].via_facts[0];
        let prov = &lowered.kb.fact(fid).expect("row fact exists").provenance;
        assert_eq!(prov.source, "Defined with respect to meter");
        assert_eq!(prov.trust_tier, TrustTier::Authoritative);
    }

    #[test]
    fn table_absent_key_has_no_proof() {
        let src = r#"
            table length_to_metres {
                columns unit, metres
                row (foot, 0.3048)
                source "Defined with respect to meter"
                trust authoritative
            }
            ? length_to_metres(furlong, $Metres)
        "#;
        let lowered = compile(src).unwrap();
        let dag = enumerate_all(&lowered.queries[0], &lowered.kb);
        assert!(dag.proofs.is_empty(), "a key not in the table abstains");
    }

    #[test]
    fn table_string_cell_lowers_to_a_string_term() {
        // A quoted cell lowers to a Str term (not an atom), so a label column is
        // representable alongside numeric factors.
        let src = r#"
            table unit_symbol {
                columns unit, symbol
                row (metre, "m")
                source "SI base unit of length"
                trust authoritative
            }
            ? unit_symbol(metre, $Symbol)
        "#;
        let lowered = compile(src).unwrap();
        let query = &lowered.queries[0];
        let v = query_var(query);
        let dag = enumerate_all(query, &lowered.kb);
        assert_eq!(dag.proofs.len(), 1);
        assert_eq!(
            dag.proofs[0].bindings.walk_var(&v),
            CoreTerm::Str("m".into())
        );
    }

    #[test]
    fn table_row_arity_mismatch_is_a_clean_error() {
        let src = r#"
            table length_to_metres {
                columns unit, metres
                row (foot, 0.3048, extra)
                source "Defined with respect to meter"
                trust authoritative
            }
        "#;
        let err = compile(src).unwrap_err();
        assert!(
            matches!(
                err,
                crate::CompileError::Lower(LowerError::TableArity {
                    expected: 2,
                    row: 0,
                    got: 3,
                    ..
                })
            ),
            "a wrong-length row is a clean TableArity error, got {err:?}"
        );
    }

    #[test]
    fn table_without_source_is_rejected() {
        let src = r#"
            table length_to_metres {
                columns unit, metres
                row (foot, 0.3048)
            }
        "#;
        let err = compile(src).unwrap_err();
        assert!(
            matches!(
                err,
                crate::CompileError::Lower(LowerError::TableMissingProvenance { .. })
            ),
            "an unsourced table is rejected (the write gate), got {err:?}"
        );
    }

    #[test]
    fn rule_derives_a_head_from_body_facts() {
        // The keystone: a `rule { head: … when: … }` lets the ENGINE DERIVE the head
        // when its body holds — so domain rulebooks (contraindications, step-therapy)
        // live in ADJ and the engine derives consequences from per-case facts, no host
        // code. Here: every pregnancy-excluded drug is derived contraindicated.
        let src = r#"
            relate pregnant(present)
            relate pregnancy_excludes(moxifloxacin)
            relate pregnancy_excludes(tmp_smx)
            rule { head: contraindicated($D) when: pregnant(present), pregnancy_excludes($D) }
            ? contraindicated($X)
        "#;
        let lowered = compile(src).unwrap();
        let query = &lowered.queries[0];
        let v = query_var(query);
        let dag = enumerate_all(query, &lowered.kb);
        assert_eq!(
            dag.proofs.len(),
            2,
            "both pregnancy-excluded drugs are derived"
        );
        assert!(dag
            .proofs
            .iter()
            .any(|p| p.bindings.walk_var(&v) == core_atom("moxifloxacin")));
        assert!(dag
            .proofs
            .iter()
            .any(|p| p.bindings.walk_var(&v) == core_atom("tmp_smx")));
    }

    #[test]
    fn rule_negation_as_failure_excludes_when_the_negated_goal_holds() {
        // `not <lit>` is negation-as-failure: `safe(D)` holds only for a β-lactam that
        // is NOT (derivably) contraindicated. ampicillin is contraindicated → not-safe;
        // ceftriaxone is the only safe drug.
        let src = r#"
            relate betalactam(ceftriaxone)
            relate betalactam(ampicillin)
            relate contraindicated(ampicillin)
            rule { head: safe($D) when: betalactam($D), not contraindicated($D) }
            ? safe($X)
        "#;
        let lowered = compile(src).unwrap();
        let query = &lowered.queries[0];
        let v = query_var(query);
        let dag = enumerate_all(query, &lowered.kb);
        assert_eq!(
            dag.proofs.len(),
            1,
            "only the non-contraindicated β-lactam is safe"
        );
        assert_eq!(
            dag.proofs[0].bindings.walk_var(&v),
            core_atom("ceftriaxone")
        );
    }

    #[test]
    fn rule_carries_its_grounding_provenance() {
        // A grounded rule keeps its citation (byte-quote + trust), so a CAS rulebook is
        // byte-traceable — the same provenance contract `relate` edges carry.
        let src = r#"
            rule { head: contraindicated($D) when: pregnant(present), excludes($D)
                   source "Pregnancy contraindicates fluoroquinolones (FDA label)."
                   trust authoritative }
            relate pregnant(present)
            relate excludes(moxifloxacin)
            ? contraindicated($X)
        "#;
        let lowered = compile(src).unwrap();
        let dag = enumerate_all(&lowered.queries[0], &lowered.kb);
        assert_eq!(dag.proofs.len(), 1);
    }

    #[test]
    fn entity_and_relation_define_kinds_parse_and_lower() {
        let src = r#"
            dictionary biochem {
                define disease : entity
                define enzyme : entity
                define deficient_in : relation from disease to enzyme
            }
        "#;
        let lowered = compile(src).unwrap();
        let kinds: Vec<&DefineKind> = lowered.dictionary.iter().map(|d| &d.kind).collect();
        assert!(kinds.iter().any(|k| matches!(k, DefineKind::Entity)));
        assert!(kinds
            .iter()
            .any(|k| matches!(k, DefineKind::Relation { from, to } if from == "disease" && to == "enzyme")));
    }

    #[test]
    fn repeated_query_variable_binds_consistently() {
        // same($A, $A) only matches an edge whose two args agree.
        let src = r#"
            relate same(x, x) trust authoritative
            relate same(x, y) trust authoritative
            ? same($A, $A)
        "#;
        let lowered = compile(src).unwrap();
        let query = &lowered.queries[0];
        let v = query_var(query);
        let dag = enumerate_all(query, &lowered.kb);
        assert_eq!(
            dag.proofs.len(),
            1,
            "only the (x, x) edge satisfies same($A, $A)"
        );
        assert_eq!(dag.proofs[0].bindings.walk_var(&v), core_atom("x"));
    }

    #[test]
    fn lowers_full_acs_rulebook_and_reproduces_adj36_posterior() {
        let src = r#"
            prior 0.10 for acs
              source "Pope JH et al., NEJM 1995;342(16):1163-70"

            contributes 1.5 from pmh(hypertension) to acs
              source "HEART Score; Six AJ et al., Neth Heart J 2008"
              trust empirical

            contributes 1.8 from pmh(smoker) to acs
              source "HEART Score; Six AJ et al., Neth Heart J 2008"
              trust empirical

            contributes 2.5 from symptom_quality(pressure_like) to acs
              source "Panju AA et al., JAMA 1998;280(14):1256-63"

            contributes 2.0 from associated_symptom(diaphoresis) to acs
              source "Panju AA et al., JAMA 1998"

            contributes 0.5 from vital_signs(within_normal_limits) to acs
              source "Panju AA et al., JAMA 1998"

            contributes 0.4 from denied(ecg_acute_st_changes) to acs
              source "Pope JH et al., NEJM 1995"

            interacts 1.3 when symptom_quality(pressure_like)
                           and associated_symptom(diaphoresis)
                           for acs
              source "[empirical] synergy"
              trust empirical

            observe pmh(hypertension)
            observe pmh(smoker)
            observe symptom_quality(pressure_like)
            observe associated_symptom(diaphoresis)
            observe vital_signs(within_normal_limits)
            observe denied(ecg_acute_st_changes)

            ? acs
        "#;
        let lowered = compile(src).unwrap();
        assert_eq!(lowered.queries.len(), 1);
        let query = &lowered.queries[0];
        let result = search(query, &lowered.kb, SearchMode::LRAggregate);
        match result {
            SearchResult::LRAggregateResult { posterior, .. } => {
                assert!(
                    (posterior - 0.281).abs() < 0.005,
                    "expected ≈0.281, got {posterior}"
                );
            }
            other => panic!("expected LRAggregateResult, got {other:?}"),
        }
    }

    #[test]
    fn duplicate_prior_is_rejected() {
        let src = r#"
            prior 0.10 for acs
            prior 0.20 for acs
        "#;
        let err = compile(src).unwrap_err();
        assert!(matches!(
            err,
            crate::CompileError::Lower(LowerError::DuplicatePrior { .. })
        ));
    }

    #[test]
    fn duplicate_source_annotation_is_rejected() {
        let src = r#"
            contributes 1.5 from pmh(htn) to acs
              source "first"
              source "second"
        "#;
        let err = compile(src).unwrap_err();
        assert!(matches!(
            err,
            crate::CompileError::Lower(LowerError::DuplicateAnnotation { name: "source" })
        ));
    }

    #[test]
    fn source_without_trust_defaults_to_authoritative() {
        let src = r#"
            contributes 1.5 from x to y
              source "some paper"
        "#;
        let lowered = compile(src).unwrap();
        let contribs = lowered.kb.contributions_for(&core_atom("y"));
        assert_eq!(contribs.len(), 1);
        assert_eq!(contribs[0].provenance.trust_tier, TrustTier::Authoritative);
    }

    #[test]
    fn no_source_and_no_trust_defaults_to_unattributed() {
        let src = "contributes 1.5 from x to y";
        let lowered = compile(src).unwrap();
        let contribs = lowered.kb.contributions_for(&core_atom("y"));
        assert_eq!(contribs[0].provenance.trust_tier, TrustTier::Unattributed);
    }

    #[test]
    fn cites_lowers_to_corroborations_in_order() {
        // ADJ-A9: `cites "<src>" locator "<loc>"` is repeatable and accumulates
        // onto the clause's Provenance::corroborations without disturbing the
        // primary source/locator/trust.
        let src = r#"
            contributes 2.5 from neutrophilia to bacterial_meningitis
              source "Tunkel 2004"
              locator "IDSA §3.2"
              trust authoritative
              cites "van de Beek 2006" locator "https://nejm.org/a"
              cites "Brouwer 2010" locator "https://asm.org/b"
        "#;
        let lowered = compile(src).unwrap();
        let contribs = lowered
            .kb
            .contributions_for(&core_atom("bacterial_meningitis"));
        assert_eq!(contribs.len(), 1);
        let p = &contribs[0].provenance;
        // Primary citation untouched.
        assert_eq!(p.source, "Tunkel 2004");
        assert_eq!(p.locator.as_deref(), Some("IDSA §3.2"));
        assert_eq!(p.trust_tier, TrustTier::Authoritative);
        // Corroborations accumulate in source order, each with its locator.
        assert_eq!(p.corroborations.len(), 2);
        assert_eq!(p.corroborations[0].source, "van de Beek 2006");
        assert_eq!(p.corroborations[0].locator, "https://nejm.org/a");
        assert_eq!(p.corroborations[1].source, "Brouwer 2010");
        assert_eq!(p.corroborations[1].locator, "https://asm.org/b");
    }

    #[test]
    fn cites_repeats_freely_unlike_at_most_once_source() {
        // A clause with NO corroborations still lowers (the common case).
        let plain = compile("contributes 1.5 from x to y\n  source \"p\"").unwrap();
        assert!(plain.kb.contributions_for(&core_atom("y"))[0]
            .provenance
            .corroborations
            .is_empty());
        // But a SECOND `source` is still rejected — only `cites` repeats.
        let err = compile("prior 0.1 for y\n  source \"a\"\n  source \"b\"").unwrap_err();
        assert!(matches!(
            err,
            crate::CompileError::Lower(LowerError::DuplicateAnnotation { name: "source" })
        ));
    }

    #[test]
    fn observe_without_query_still_compiles() {
        let src = "observe pmh(hypertension)";
        let lowered = compile(src).unwrap();
        assert_eq!(lowered.queries.len(), 0);
    }

    #[test]
    fn uncertain_statement_produces_voi_report_on_aggregation() {
        // The ACS rulebook with a `uncertain {…}` clause for the
        // precipitator, no precipitator observation. The aggregator
        // should return a VOI report listing the three candidate
        // values and the maximum log-odds swing knowing one of
        // them would produce.
        let src = r#"
            prior 0.10 for acs

            contributes 1.5 from pmh(hypertension) to acs
            contributes 2.5 from precipitator(exertional) to acs
            contributes 0.6 from precipitator(rest) to acs
            contributes 0.8 from precipitator(positional) to acs

            observe pmh(hypertension)

            uncertain { precipitator(exertional),
                        precipitator(rest),
                        precipitator(positional) } for acs
              source "patient did not specify"

            ? acs
        "#;
        let lowered = compile(src).unwrap();
        let query = &lowered.queries[0];
        let result = search(query, &lowered.kb, SearchMode::LRAggregate);
        match result {
            SearchResult::LRAggregateResult { uncertainties, .. } => {
                assert_eq!(uncertainties.len(), 1);
                let report = &uncertainties[0];
                assert_eq!(report.domain.len(), 3);
                // VOI = ln(2.5) - ln(0.6) ≈ 1.4271
                assert!(
                    (report.voi_logit_range - (2.5_f64.ln() - 0.6_f64.ln())).abs() < 1e-9,
                    "got VOI {}",
                    report.voi_logit_range
                );
            }
            other => panic!("expected LRAggregateResult, got {other:?}"),
        }
    }

    #[test]
    fn negative_contributes_lr_is_a_clean_error_not_a_panic() {
        // Regression: a malformed rulebook must not panic the process.
        // `contributes -5 ...` would hit the engine's `assert!(lr > 0.0)`.
        for src in [
            "contributes -5 from x to y",
            "contributes 0 from x to y",
            "interacts -1 when a and b for y",
        ] {
            let err = compile(src).unwrap_err();
            assert!(
                matches!(
                    err,
                    crate::CompileError::Lower(LowerError::InvalidLikelihoodRatio { .. })
                ),
                "expected InvalidLikelihoodRatio for {src:?}, got {err:?}"
            );
        }
    }

    #[test]
    fn out_of_range_prior_is_a_clean_error_not_a_panic() {
        for src in ["prior 2 for x", "prior 0 for x", "prior -0.5 for x"] {
            let err = compile(src).unwrap_err();
            assert!(
                matches!(
                    err,
                    crate::CompileError::Lower(LowerError::InvalidProbability { .. })
                ),
                "expected InvalidProbability for {src:?}, got {err:?}"
            );
        }
    }

    #[test]
    fn large_magnitude_literal_is_stored_exactly_not_rejected() {
        // Before NX-2, `1e400` overflowed `f64` to +inf and was rejected at parse. Now it is a
        // perfectly good exact decimal (`10^400`) preserved with every digit — the whole point of
        // the exact-numbers arc is that a written magnitude is stored as written, not truncated
        // (or, here, saturated to inf) the instant it is parsed. The lossy `f64` view appears only
        // if something later asks for one.
        let src = r#"
            observe gross_income(1e400)
            ? gross_income($V)
        "#;
        let lowered = compile(src).unwrap();
        let query = &lowered.queries[0];
        let v = query_var(query);
        let dag = enumerate_all(query, &lowered.kb);
        assert_eq!(dag.proofs.len(), 1);
        assert_eq!(
            dag.proofs[0].bindings.walk_var(&v),
            CoreTerm::Num(CoreNumber::Exact(
                bignum_core::BigDecimal::from_str("1e400").unwrap()
            ))
        );
    }

    #[test]
    fn predicate_gated_contribution_fires_end_to_end() {
        // A DETERMINISTIC rule as a saturating LR: "income at/above the
        // filing threshold ⇒ required to file." The model authored the
        // rulebook; the comparison runs in the engine at decision time.
        let src = r#"
            prior 0.10 for required_to_file
            contributes 1000000 from gross_income >= 14600 to required_to_file
              source "IRS Pub 501 (2024), filing threshold single < 65"
              trust authoritative
            observe gross_income(18000)
            ? required_to_file
        "#;
        let lowered = compile(src).unwrap();
        let query = &lowered.queries[0];
        let result = search(query, &lowered.kb, SearchMode::LRAggregate);
        match result {
            SearchResult::LRAggregateResult { posterior, dag, .. } => {
                assert!(posterior > 0.9999, "should saturate, got {posterior}");
                // The proof carries the literal comparison that fired.
                let fired = dag.proofs[0].steps.iter().any(|s| {
                    matches!(
                        s.origin,
                        logic_engine::DerivationOrigin::FromPredicateContribution { .. }
                    )
                });
                assert!(fired, "expected a predicate-contribution step");
            }
            other => panic!("expected LRAggregateResult, got {other:?}"),
        }
    }

    #[test]
    fn predicate_fires_over_typed_value_literal_end_to_end() {
        // Step 2: typed value literals. `quantity(18000, usd)` already
        // parses as a nested compound under the predicate grammar; the
        // engine reads its leading magnitude (18000) for the predicate
        // while the `usd` unit travels with the fact. No grammar change.
        let src = r#"
            prior 0.10 for required_to_file
            contributes 1000000 from gross_income >= 14600 to required_to_file
              source "IRS Pub 501 (2024)" trust authoritative
            observe gross_income(quantity(18000, usd))
            ? required_to_file
        "#;
        let lowered = compile(src).unwrap();
        let query = &lowered.queries[0];
        match search(query, &lowered.kb, SearchMode::LRAggregate) {
            SearchResult::LRAggregateResult { posterior, .. } => {
                assert!(posterior > 0.9999, "should saturate, got {posterior}");
            }
            other => panic!("expected LRAggregateResult, got {other:?}"),
        }
    }

    #[test]
    fn predicate_below_threshold_stays_at_prior() {
        let src = r#"
            prior 0.10 for required_to_file
            contributes 1000000 from gross_income >= 14600 to required_to_file
            observe gross_income(9000)
            ? required_to_file
        "#;
        let lowered = compile(src).unwrap();
        let query = &lowered.queries[0];
        match search(query, &lowered.kb, SearchMode::LRAggregate) {
            SearchResult::LRAggregateResult { posterior, .. } => {
                assert!((posterior - 0.10).abs() < 1e-9, "got {posterior}");
            }
            other => panic!("expected LRAggregateResult, got {other:?}"),
        }
    }

    #[test]
    fn locator_annotation_is_carried_through() {
        let src = r#"
            contributes 1.5 from x to y
              source "guideline"
              locator "§3.2"
        "#;
        let lowered = compile(src).unwrap();
        let contribs = lowered.kb.contributions_for(&core_atom("y"));
        assert_eq!(contribs[0].provenance.locator.as_deref(), Some("§3.2"));
    }

    // ---- `let` + arithmetic (ADJ expansion step 3b) ----

    #[test]
    fn let_arithmetic_computes_a_ratio_with_a_cited_tree() {
        let src = r#"
            observe csf_glucose(quantity(40, mg_dl))
            observe serum_glucose(quantity(100, mg_dl))
            let csf_ratio = csf_glucose / serum_glucose
        "#;
        let lowered = compile(src).unwrap();
        let d = lowered
            .kb
            .derived_for("csf_ratio")
            .expect("csf_ratio should be bound");
        assert!((d.value - 0.4).abs() < 1e-12, "got {}", d.value);
    }

    #[test]
    fn let_derived_value_fires_a_predicate_end_to_end() {
        // The whole point: a predicate fires over a COMPUTED value exactly
        // as over an observed one. Low CSF:serum ratio ⇒ bacterial.
        let src = r#"
            prior 0.30 for bacterial
            observe csf_glucose(40)
            observe serum_glucose(100)
            let csf_ratio = csf_glucose / serum_glucose
            contributes 1000000 from csf_ratio <= 0.5 to bacterial
              source "Spanos 1989" trust authoritative
            ? bacterial
        "#;
        let lowered = compile(src).unwrap();
        let query = &lowered.queries[0];
        match search(query, &lowered.kb, SearchMode::LRAggregate) {
            SearchResult::LRAggregateResult { posterior, .. } => {
                assert!(
                    posterior > 0.9999,
                    "predicate over derived value should fire; got {posterior}"
                );
            }
            other => panic!("expected LRAggregateResult, got {other:?}"),
        }
    }

    #[test]
    fn predicate_rhs_expression_fires_over_fraction_result() {
        let d = crate::compile_and_decide(
            r#"let answer = 1 / 10 + 2 / 10
prior 0.10 for opt_a
contributes 1000000 from answer == 3 / 10 to opt_a
? opt_a
"#,
        )
        .unwrap();
        assert!(d.ranked[0].posterior > 0.99, "{d:?}");
    }

    #[test]
    fn let_sum_aggregates_repeated_observations() {
        let src = r#"
            observe line_item(12000)
            observe line_item(6000)
            observe line_item(2000)
            let total = sum(line_item)
        "#;
        let lowered = compile(src).unwrap();
        assert!((lowered.kb.derived_for("total").unwrap().value - 20000.0).abs() < 1e-9);
    }

    #[test]
    fn let_respects_operator_precedence_and_parens() {
        // a + b * c  ==  a + (b*c);  (a + b) * c  forces the other grouping.
        let src = r#"
            observe a(2)
            observe b(3)
            observe c(4)
            let unparen = a + b * c
            let paren = (a + b) * c
        "#;
        let lowered = compile(src).unwrap();
        assert_eq!(lowered.kb.derived_for("unparen").unwrap().value, 14.0); // 2 + 12
        assert_eq!(lowered.kb.derived_for("paren").unwrap().value, 20.0); //  5 * 4
    }

    #[test]
    fn let_can_reference_an_earlier_let() {
        let src = r#"
            observe a(3)
            observe b(4)
            let s = a + b
            let d = s * 2
        "#;
        let lowered = compile(src).unwrap();
        assert_eq!(lowered.kb.derived_for("d").unwrap().value, 14.0);
    }

    #[test]
    fn let_over_unknown_slot_is_a_clean_error() {
        let err = compile("let x = nope / 2").unwrap_err();
        assert!(
            matches!(
                err,
                crate::CompileError::Lower(LowerError::ComputationFailed { .. })
            ),
            "got {err:?}"
        );
    }

    // ---- constraint sublanguage (ADJ constraints track B1) ----

    #[test]
    fn symbol_constrain_solve_check_build_a_constraint_system() {
        // A small eligibility set: premium is unknown, bounded above by 2000
        // and below by the observed base_rate; solve for it.
        let src = r#"
            symbol premium : money(usd)
            symbol months  : scalar
            observe base_rate(1200)
            constrain premium <= 2000
            constrain premium >= base_rate
            constrain months >= 6
            solve for { premium, months }
        "#;
        let lowered = compile(src).unwrap();
        let cs = &lowered.constraints;
        assert!(!cs.is_empty());
        assert_eq!(cs.symbols.len(), 2);
        assert_eq!(cs.symbols[0].0, "premium");
        assert!(matches!(
            &cs.symbols[0].1,
            core_compound_money if format!("{core_compound_money:?}").contains("money")
        ));
        assert_eq!(cs.symbols[1].0, "months");
        assert_eq!(cs.constraints.len(), 3);
        assert_eq!(cs.constraints[0].op, crate::ast::RelOp::Le);
        assert_eq!(cs.constraints[1].op, crate::ast::RelOp::Ge);
        assert_eq!(
            cs.solve_for,
            vec!["premium".to_string(), "months".to_string()]
        );
        assert!(!cs.check);
    }

    #[test]
    fn check_sets_the_feasibility_flag() {
        let lowered = compile("constrain x >= 1\ncheck").unwrap();
        assert!(lowered.constraints.check);
        assert_eq!(lowered.constraints.constraints.len(), 1);
    }

    // ---- dictionary + define (MYCIN-2026 M1) ----

    #[test]
    fn a_dictionary_compiles_and_defined_terms_are_accepted() {
        let src = "dictionary v {\n\
                   define dx : hypothesis surface \"the diagnosis\"\n\
                   define f : finding values [a, b] surface \"finding f\"\n\
                   }\n\
                   prior 0.10 for dx\n  source \"x\" trust empirical\n\
                   contributes 2 from f(a) to dx\n  source \"y\" trust empirical\n\
                   observe f(a)\n? dx\n";
        let lowered = compile(src).unwrap();
        assert_eq!(lowered.dictionary.len(), 2);
        // surface forms are captured
        let f = lowered.dictionary.iter().find(|d| d.name == "f").unwrap();
        assert_eq!(f.surfaces, vec!["finding f".to_string()]);
        assert!(
            matches!(&f.kind, crate::ast::DefineKind::Finding { values } if values == &["a", "b"])
        );
    }

    #[test]
    fn a_bare_define_outside_a_block_also_registers() {
        let lowered = compile("define dx : hypothesis\n? dx\n").unwrap();
        assert_eq!(lowered.dictionary.len(), 1);
    }

    #[test]
    fn a_valid_pinned_quote_lowers_to_a_verbatim_span_and_snapshot() {
        // RS-4 PR-D4a: quote/at/snapshot populates Provenance.quote + .snapshot.
        let hex = "0".repeat(64);
        let src = format!(
            "relate inhibits(aspirin, cyclooxygenase)\n    \
             quote \"Aspirin inhibits cyclooxygenase\" at 7 snapshot \"{hex}\"\n    \
             source \"ref\"\n? inhibits(aspirin, $X)\n"
        );
        let lowered = compile(&src).unwrap();
        let query = &lowered.queries[0];
        let dag = enumerate_all(query, &lowered.kb);
        assert_eq!(dag.proofs.len(), 1, "the relate fact matches the query");
        let fid = dag.proofs[0].via_facts[0];
        let prov = &lowered.kb.fact(fid).expect("relate fact exists").provenance;
        assert_eq!(
            prov.quote.text(),
            Some("Aspirin inhibits cyclooxygenase"),
            "the verbatim span is carried through"
        );
        assert!(prov.snapshot.is_some(), "and the snapshot hash is pinned");
    }

    #[test]
    fn a_malformed_snapshot_hash_is_a_fail_closed_compile_error() {
        let src = "relate inhibits(a, b)\n    \
                   quote \"hi\" at 0 snapshot \"nothex\"\n    source \"ref\"\n? inhibits(a, $X)\n";
        let err = compile(src).unwrap_err();
        assert!(
            matches!(
                err,
                crate::CompileError::Lower(LowerError::MalformedQuotePin { .. })
            ),
            "a non-SHA-256 snapshot must be a compile error, not a half-built pin: {err:?}"
        );
    }

    #[test]
    fn a_blank_quote_text_is_a_fail_closed_compile_error() {
        let hex = "0".repeat(64);
        let src = format!(
            "relate inhibits(a, b)\n    quote \"   \" at 0 snapshot \"{hex}\"\n    \
             source \"ref\"\n? inhibits(a, $X)\n"
        );
        let err = compile(&src).unwrap_err();
        assert!(
            matches!(
                err,
                crate::CompileError::Lower(LowerError::MalformedQuotePin { .. })
            ),
            "a quote with no visible content cannot anchor anything: {err:?}"
        );
    }

    #[test]
    fn an_undefined_hypothesis_is_a_clean_error() {
        let err =
            compile("define f : finding values [a]\nobserve f(a)\n? undefined_dx\n").unwrap_err();
        assert!(
            matches!(err, crate::CompileError::Lower(LowerError::UndefinedTerm { ref name, expected: "hypothesis" }) if name == "undefined_dx"),
            "{err:?}"
        );
    }

    #[test]
    fn an_undefined_finding_is_a_clean_error() {
        let err = compile("define dx : hypothesis\nobserve mystery(x)\n? dx\n").unwrap_err();
        assert!(
            matches!(err, crate::CompileError::Lower(LowerError::UndefinedTerm { ref name, expected: "finding" }) if name == "mystery"),
            "{err:?}"
        );
    }

    #[test]
    fn a_value_outside_the_domain_is_rejected() {
        let err = compile(
            "dictionary v { define dx : hypothesis  define f : finding values [a, b] }\n\
             observe f(c)\n? dx\n",
        )
        .unwrap_err();
        assert!(
            matches!(err, crate::CompileError::Lower(LowerError::ValueNotInDomain { ref functor, ref value, .. }) if functor == "f" && value == "c"),
            "{err:?}"
        );
    }

    #[test]
    fn no_dictionary_means_no_enforcement() {
        // Backward-compatible: a program with no `define` is unchecked.
        let lowered =
            compile("prior 0.10 for anything\n  source \"x\" trust empirical\n? anything\n")
                .unwrap();
        assert!(lowered.dictionary.is_empty());
    }

    // ---- rulebook + use (MYCIN-2026 M2) ----

    #[test]
    fn a_rulebook_lowers_its_clauses_into_the_kb_like_top_level() {
        // The `rulebook { … }` wrapper is metadata; its clauses populate the KB
        // exactly as if written at top level, so the query still decides.
        let src = "rulebook meningitis {\n\
                   prior 0.10 for bacterial\n  source \"x\" trust empirical\n\
                   contributes 3 from csf(low) to bacterial\n  source \"y\" trust empirical\n\
                   }\n\
                   observe csf(low)\n? bacterial\n";
        let lowered = compile(src).unwrap();
        assert_eq!(lowered.queries.len(), 1);
        let d = crate::decide(&lowered);
        // one contribution fired over the prior → posterior above the 0.10 base.
        assert!(d.ranked[0].posterior > 0.10, "{d:?}");
    }

    #[test]
    fn a_rulebook_that_uses_a_dictionary_is_checked_against_it() {
        // Every term the rulebook names is defined in the `use`d dictionary.
        let src = "dictionary vocab {\n\
                   define bacterial : hypothesis\n\
                   define csf : finding values [low, normal]\n\
                   }\n\
                   rulebook meningitis {\n\
                   use vocab\n\
                   prior 0.10 for bacterial\n  source \"x\" trust empirical\n\
                   contributes 3 from csf(low) to bacterial\n  source \"y\" trust empirical\n\
                   }\n";
        let lowered = compile(src).unwrap();
        assert_eq!(lowered.dictionary.len(), 2);
    }

    #[test]
    fn a_rulebook_naming_an_undefined_term_is_rejected() {
        // `meningococcal` is not in the `use`d dictionary → clean error, scoped
        // to the rulebook by its `use`.
        let src = "dictionary vocab {\n\
                   define bacterial : hypothesis\n\
                   define csf : finding values [low, normal]\n\
                   }\n\
                   rulebook meningitis {\n\
                   use vocab\n\
                   contributes 3 from csf(low) to meningococcal\n  source \"y\" trust empirical\n\
                   }\n";
        let err = compile(src).unwrap_err();
        assert!(
            matches!(err, crate::CompileError::Lower(LowerError::UndefinedTerm { ref name, expected: "hypothesis" }) if name == "meningococcal"),
            "{err:?}"
        );
    }

    #[test]
    fn use_of_an_undeclared_dictionary_is_a_clean_error() {
        let src = "rulebook meningitis {\n\
                   use nonexistent\n\
                   prior 0.10 for bacterial\n  source \"x\" trust empirical\n\
                   }\n";
        let err = compile(src).unwrap_err();
        assert!(
            matches!(err, crate::CompileError::Lower(LowerError::UndefinedDictionary { ref name }) if name == "nonexistent"),
            "{err:?}"
        );
    }

    #[test]
    fn a_rulebook_without_use_is_unchecked_even_when_a_dictionary_exists() {
        // A `use` elsewhere flips the program to M2 scoped mode; a rulebook that
        // declines to `use` is then unchecked — it may name terms the dictionary
        // never declared. (Backward-compatible "opt in to checking" semantics.)
        let src = "dictionary vocab { define bacterial : hypothesis }\n\
                   rulebook checked { use vocab\n prior 0.10 for bacterial\n  source \"a\" trust empirical\n }\n\
                   rulebook freeform {\n contributes 2 from whatever(x) to anything\n  source \"b\" trust empirical\n }\n";
        // Compiles despite `freeform` naming undefined terms, because it has no `use`.
        let lowered = compile(src).unwrap();
        assert_eq!(lowered.dictionary.len(), 1);
    }

    #[test]
    fn a_nested_rulebook_is_rejected() {
        // Rulebooks are flat containers — nesting has no defined scoping
        // semantics and is refused cleanly (also bounds flatten recursion).
        let src = "rulebook outer {\n\
                   rulebook inner {\n\
                   prior 0.10 for bacterial\n  source \"x\" trust empirical\n\
                   }\n\
                   }\n? bacterial\n";
        let err = compile(src).unwrap_err();
        assert!(
            matches!(err, crate::CompileError::Lower(LowerError::NestedRulebook { ref outer, ref inner }) if outer == "outer" && inner == "inner"),
            "{err:?}"
        );
    }

    #[test]
    fn a_top_level_use_scopes_the_top_level_clauses() {
        let src = "dictionary vocab { define bacterial : hypothesis  define csf : finding values [low] }\n\
                   use vocab\n\
                   contributes 3 from csf(low) to bacterial\n  source \"y\" trust empirical\n\
                   observe csf(low)\n? bacterial\n";
        let lowered = compile(src).unwrap();
        assert_eq!(lowered.queries.len(), 1);
    }

    #[test]
    fn a_top_level_use_rejects_an_undefined_top_level_term() {
        let src = "dictionary vocab { define bacterial : hypothesis }\n\
                   use vocab\n\
                   observe mystery(x)\n? bacterial\n";
        let err = compile(src).unwrap_err();
        assert!(
            matches!(err, crate::CompileError::Lower(LowerError::UndefinedTerm { ref name, expected: "finding" }) if name == "mystery"),
            "{err:?}"
        );
    }

    #[test]
    fn maximize_sets_an_lp_objective() {
        // `maximize` lowers to an Optimize objective kept as an unevaluated
        // ComputeExpr (it mentions the symbols the LP solver assigns).
        let lowered = compile("symbol x : scalar\nconstrain x <= 5\nmaximize x + 1").unwrap();
        let cs = &lowered.constraints;
        assert!(!cs.is_empty());
        let (dir, obj) = cs.objective.as_ref().expect("an objective");
        assert_eq!(*dir, crate::ast::OptDir::Maximize);
        // x + 1 is a Bin(Add, Ref(x), Lit(1)) — unevaluated.
        assert!(format!("{obj:?}").contains("Add"), "{obj:?}");
    }

    #[test]
    fn minimize_sets_the_direction() {
        let lowered = compile("symbol x : scalar\nconstrain x >= 3\nminimize x").unwrap();
        let (dir, _) = lowered
            .constraints
            .objective
            .as_ref()
            .expect("an objective");
        assert_eq!(*dir, crate::ast::OptDir::Minimize);
    }

    #[test]
    fn constraint_operands_lower_to_unevaluated_compute_exprs() {
        // `constrain total = a + b * c` — the rhs stays a ComputeExpr tree
        // (not evaluated; it mentions symbols the solver will assign).
        let lowered = compile("constrain total = a + b * 2").unwrap();
        let c = &lowered.constraints.constraints[0];
        assert_eq!(c.op, crate::ast::RelOp::Eq);
        assert!(matches!(c.lhs, logic_engine::ComputeExpr::Ref(_)));
        // rhs is a + (b * 2): an Add whose right operand is a Mul.
        assert!(matches!(
            c.rhs,
            logic_engine::ComputeExpr::Bin(logic_engine::ComputeOp::Add, _, _)
        ));
    }

    #[test]
    fn native_latex_expr_computes_inside_let() {
        let d = crate::compile_and_decide(
            r#"let answer = latex "$5 \times 12$"
prior 0.10 for correct
contributes 1000000 from answer == 60 to correct
? correct
"#,
        )
        .unwrap();
        assert!(d.ranked[0].posterior > 0.99, "{d:?}");
    }

    // --- AsciiMath: a SECOND math frontend on the same neutral pipeline (PFE01) ---
    // These prove that `asciimath "..."` reaches the identical `MathExpr -> ExprAst`
    // lowering the `latex "..."` surface uses: the ONLY difference is which
    // MathFrontend parses the string. So the whole arithmetic subset computes
    // through `compile_and_decide` for free, with no new lowering or engine op.

    #[test]
    fn native_asciimath_expr_computes_inside_let() {
        // `(3+4)*2` = 14. AsciiMath `*` is the same Mul the LaTeX `\times`/`\cdot`
        // lowered to, so the engine derives the same scalar.
        let d = crate::compile_and_decide(
            "let answer = asciimath \"(3+4)*2\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 14 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(d.ranked[0].posterior > 0.99, "{d:?}");
    }

    #[test]
    fn native_asciimath_fraction_reuses_the_latex_frac_lowering() {
        // AsciiMath `(3+4)/2` parses to the SAME `MathExpr::Frac` that LaTeX
        // `\frac{3+4}{2}` produces, so it flows through the identical lowering and
        // computes 7/2 = 3.5 — the clearest demonstration that the second frontend
        // is consumed for free by the existing neutral-tree path.
        let d = crate::compile_and_decide(
            "let answer = asciimath \"(3+4)/2\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 3.5 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(d.ranked[0].posterior > 0.99, "{d:?}");
    }

    #[test]
    fn native_asciimath_observed_symbol_binds() {
        // An observed slot referenced from inside an AsciiMath expression binds
        // exactly as it does for LaTeX: with x=2 observed, `x*x` = 4.
        let d = crate::compile_and_decide(
            "observe x(2)\n\
             let answer = asciimath \"x*x\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 4 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(d.ranked[0].posterior > 0.99, "{d:?}");
    }

    // --- MathML: a THIRD math frontend on the same neutral pipeline (PFE01) ---
    // Same proof again for presentation MathML: the ONLY difference from `latex`/
    // `asciimath` is which MathFrontend parses the string; the neutral MathExpr
    // flows through the identical lowering, so it computes for free.

    #[test]
    fn native_mathml_fraction_reuses_the_latex_frac_lowering() {
        // `<mfrac><mn>7</mn><mn>2</mn></mfrac>` parses to the SAME `MathExpr::Frac`
        // that LaTeX `\frac{7}{2}` and AsciiMath `7/2` produce, so it computes 3.5.
        let d = crate::compile_and_decide(
            "let answer = mathml \"<mfrac><mn>7</mn><mn>2</mn></mfrac>\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 3.5 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(d.ranked[0].posterior > 0.99, "{d:?}");
    }

    #[test]
    fn native_mathml_sum_computes_inside_let() {
        // `<mn>3</mn><mo>+</mo><mn>4</mn>` = 7 — the `<mo>+</mo>` operator lowers to
        // the same Add the LaTeX/AsciiMath `+` did.
        let d = crate::compile_and_decide(
            "let answer = mathml \"<mn>3</mn><mo>+</mo><mn>4</mn>\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 7 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(d.ranked[0].posterior > 0.99, "{d:?}");
    }

    #[test]
    fn native_mathml_observed_symbol_binds() {
        // An observed slot referenced from inside a MathML expression binds
        // exactly as for LaTeX/AsciiMath: with x=2 observed,
        // `<mi>x</mi><mo>*</mo><mi>x</mi>` = 4.
        let d = crate::compile_and_decide(
            "observe x(2)\n\
             let answer = mathml \"<mi>x</mi><mo>*</mo><mi>x</mi>\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 4 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(d.ranked[0].posterior > 0.99, "{d:?}");
    }

    // --- Unicode-math: a FOURTH math frontend — PFE01 quartet complete ---
    // Raw Unicode glyphs (÷, ×) parse to the SAME neutral MathExpr the other three
    // surfaces produce, flowing through the identical lowering; the ONLY difference
    // is which MathFrontend parses the string.

    #[test]
    fn native_unicodemath_division_computes_inside_let() {
        // `(3+4) ÷ 2` — the Unicode division sign lowers to the same division as
        // LaTeX `\div` / AsciiMath+MathML `/`, so it computes 7/2 = 3.5.
        let d = crate::compile_and_decide(
            "let answer = unicodemath \"(3+4) ÷ 2\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 3.5 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(d.ranked[0].posterior > 0.99, "{d:?}");
    }

    #[test]
    fn native_unicodemath_times_computes_inside_let() {
        // `3 × 4` — the Unicode multiplication sign is the same Mul the other
        // frontends' `*`/`×`/`\times` lowered to: = 12.
        let d = crate::compile_and_decide(
            "let answer = unicodemath \"3 × 4\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 12 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(d.ranked[0].posterior > 0.99, "{d:?}");
    }

    #[test]
    fn native_unicodemath_observed_symbol_binds() {
        // An observed slot referenced from inside a Unicode-math expression binds
        // exactly as for the sibling surfaces: with x=2 observed, `x × x` = 4.
        let d = crate::compile_and_decide(
            "observe x(2)\n\
             let answer = unicodemath \"x × x\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 4 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(d.ranked[0].posterior > 0.99, "{d:?}");
    }

    #[test]
    fn native_latex_relation_lowers_to_constraint() {
        let lowered =
            compile("symbol x : scalar\nconstrain latex \"$x^2 = 4$\"\nsolve for { x }\n").unwrap();
        let c = &lowered.constraints.constraints[0];
        assert_eq!(c.op, crate::ast::RelOp::Eq);
        // `x^2` now lowers to a single native power node (ComputeOp::Pow), not the
        // old `x*x` expansion; the constraint solver's polynomial path still reads
        // it as a quadratic (see adj-constraint-solver `poly_of`).
        assert!(matches!(
            c.lhs,
            logic_engine::ComputeExpr::Bin(logic_engine::ComputeOp::Pow, _, _)
        ));
    }

    #[test]
    fn native_asciimath_relation_lowers_to_constraint() {
        // `constrain asciimath "x^2 = 4"` lowers through the SAME path as
        // `constrain latex`: the AsciiMath frontend yields MathExpr::Rel(Eq, ..),
        // the operator lowers via lower_latex_relop, and both sides via
        // latex_math_to_expr_ast — so `x^2` becomes the same single ComputeOp::Pow
        // node the LaTeX surface produces (proving the two constraint surfaces are
        // one code path with the frontend swapped).
        let lowered =
            compile("symbol x : scalar\nconstrain asciimath \"x^2 = 4\"\nsolve for { x }\n")
                .unwrap();
        let c = &lowered.constraints.constraints[0];
        assert_eq!(c.op, crate::ast::RelOp::Eq);
        assert!(matches!(
            c.lhs,
            logic_engine::ComputeExpr::Bin(logic_engine::ComputeOp::Pow, _, _)
        ));
    }

    #[test]
    fn native_asciimath_inequality_relation_lowers() {
        // A non-equality AsciiMath relation (`a <= b`) lowers to the matching
        // engine RelOp, confirming lower_latex_relop is reused verbatim — the
        // AsciiMath surface is not limited to equalities.
        let lowered = compile(
            "symbol a : scalar\nsymbol b : scalar\nconstrain asciimath \"a <= b\"\ncheck\n",
        )
        .unwrap();
        let c = &lowered.constraints.constraints[0];
        assert_eq!(c.op, crate::ast::RelOp::Le);
    }

    #[test]
    fn native_latex_power_computes_as_one_node_without_the_old_cap() {
        // `x^{10}` used to be rejected (expansion capped at exponent 8); it now
        // lowers to a single ComputeOp::Pow node and computes: 2^10 = 1024.
        // (A single-letter symbol — LaTeX math reads `base` as b·a·s·e.)
        let d = crate::compile_and_decide(
            "observe x(2)\n\
             let answer = latex \"$x^{10}$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 1024 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(d.ranked[0].posterior > 0.99, "{d:?}");
    }

    #[test]
    fn native_latex_symbolic_exponent_binds() {
        // `x^y` — a SYMBOLIC exponent now lowers (base AND exponent are general
        // expressions): with x=2, y=3 observed, `x^y` = 2^3 = 8. Previously the
        // adapter required a non-negative integer literal exponent and rejected
        // `x^y`; the engine's ComputeOp::Pow evaluates the exponent at run time.
        let sy = crate::compile_and_decide(
            "observe x(2)\n\
             observe y(3)\n\
             let answer = latex \"$x^y$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 8 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(sy.ranked[0].posterior > 0.99, "{sy:?}");
        // A COMPUTED exponent: `x^{a+b}` with x=2, a=1, b=2 → 2^3 = 8.
        let comp = crate::compile_and_decide(
            "observe x(2)\n\
             observe a(1)\n\
             observe b(2)\n\
             let answer = latex \"$x^{a+b}$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 8 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(comp.ranked[0].posterior > 0.99, "{comp:?}");
    }

    #[test]
    fn native_latex_square_root_computes() {
        // `\sqrt{9}` lowers to `9 ^ 0.5` (reusing ComputeOp::Pow) and computes 3
        // for a dimensionless base.
        let d = crate::compile_and_decide(
            "let answer = latex \"$\\sqrt{9}$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 3 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(d.ranked[0].posterior > 0.99, "{d:?}");
    }

    #[test]
    fn native_latex_square_root_of_an_observed_scalar_computes() {
        // `\sqrt{x}` over an observed dimensionless value: √16 = 4.
        let d = crate::compile_and_decide(
            "observe x(16)\n\
             let answer = latex \"$\\sqrt{x}$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 4 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(d.ranked[0].posterior > 0.99, "{d:?}");
    }

    #[test]
    fn native_latex_cube_root_computes() {
        // `\sqrt[3]{27}` lowers to `27 ^ (1/3)` (reusing ComputeOp::Pow) and
        // computes 3 for a dimensionless base — the nth-root slice on top of the
        // square root: a degree present emits the reciprocal exponent `1/n`.
        let d = crate::compile_and_decide(
            "let answer = latex \"$\\sqrt[3]{27}$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 3 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(d.ranked[0].posterior > 0.99, "{d:?}");
    }

    #[test]
    fn native_latex_fourth_root_of_an_observed_scalar_computes() {
        // `\sqrt[4]{x}` over an observed dimensionless value: the fourth root of
        // 16 is 2 (16 ^ (1/4) = 2).
        let d = crate::compile_and_decide(
            "observe x(16)\n\
             let answer = latex \"$\\sqrt[4]{x}$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 2 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(d.ranked[0].posterior > 0.99, "{d:?}");
    }

    #[test]
    fn native_latex_absolute_value_of_a_negative_difference_computes() {
        // `|a - b|` with a < b is a negative difference; the absolute value flips
        // it positive. `|3 - 10| = 7`, computed on the native ComputeOp::Abs —
        // previously the `|…|` bars were silently dropped and it computed `-7`.
        let d = crate::compile_and_decide(
            "observe a(3)\n\
             observe b(10)\n\
             let answer = latex \"$\\left|a - b\\right|$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 7 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(d.ranked[0].posterior > 0.99, "{d:?}");
    }

    #[test]
    fn native_latex_absolute_value_of_a_positive_value_is_unchanged() {
        // `|x|` of an already-positive value returns it unchanged: `|5| = 5`.
        let d = crate::compile_and_decide(
            "observe x(5)\n\
             let answer = latex \"$\\left|x\\right|$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 5 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(d.ranked[0].posterior > 0.99, "{d:?}");
    }

    #[test]
    fn native_latex_floor_rounds_a_quotient_down() {
        // `⌊a / b⌋` with a=7, b=2 is ⌊3.5⌋ = 3 (the greatest integer ≤ 3.5),
        // computed on the native ComputeOp::Floor — the `\lfloor…\rfloor` fence
        // is honoured, not silently dropped to the bare quotient 3.5.
        let d = crate::compile_and_decide(
            "observe a(7)\n\
             observe b(2)\n\
             let answer = latex \"$\\left\\lfloor a / b\\right\\rfloor$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 3 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(d.ranked[0].posterior > 0.99, "{d:?}");
    }

    #[test]
    fn native_latex_ceiling_rounds_a_quotient_up() {
        // `⌈a / b⌉` with a=7, b=2 is ⌈3.5⌉ = 4 (the least integer ≥ 3.5),
        // computed on the native ComputeOp::Ceil via the `\lceil…\rceil` fence.
        let d = crate::compile_and_decide(
            "observe a(7)\n\
             observe b(2)\n\
             let answer = latex \"$\\left\\lceil a / b\\right\\rceil$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 4 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(d.ranked[0].posterior > 0.99, "{d:?}");
    }

    #[test]
    fn native_latex_nearest_integer_fence_rounds_a_quotient() {
        // The asymmetric nearest-integer fence `⌊a / b⌉` = `\left\lfloor…\right\rceil`
        // (floor-left, ceil-right) rounds to the nearest integer, ties away from
        // zero: ⌊7/2⌉ = ⌊3.5⌉ = 4, computed on the native ComputeOp::Round — NOT the
        // floor 3 (that would be `\rfloor`), so the asymmetric delimiters matter.
        let d = crate::compile_and_decide(
            "observe a(7)\n\
             observe b(2)\n\
             let answer = latex \"$\\left\\lfloor a / b\\right\\rceil$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 4 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(d.ranked[0].posterior > 0.99, "{d:?}");
    }

    #[test]
    fn native_latex_trunc_drops_a_positive_fraction_toward_zero() {
        // `\operatorname{trunc}(a / b)` with a=7, b=2 is trunc(3.5) = 3, computed on
        // the native ComputeOp::Trunc via the operator-name juxtaposition path
        // (`Bin(Mul, Text("trunc"), (a / b))`) — NOT silently dropped to the bare
        // quotient 3.5.
        let d = crate::compile_and_decide(
            "observe a(7)\n\
             observe b(2)\n\
             let answer = latex \"$\\operatorname{trunc}(a / b)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 3 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(d.ranked[0].posterior > 0.99, "{d:?}");
    }

    #[test]
    fn native_latex_trunc_of_a_negative_quotient_goes_toward_zero_not_down() {
        // trunc(−7/2) = −3 (toward zero), the whole distinction from floor, which
        // would give −4 (toward −∞). Confirms the operator-name path reaches
        // ComputeOp::Trunc and not ComputeOp::Floor.
        let d = crate::compile_and_decide(
            "observe a(7)\n\
             observe b(2)\n\
             let answer = latex \"$\\operatorname{trunc}((0 - a) / b)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == -3 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(d.ranked[0].posterior > 0.99, "{d:?}");
    }

    #[test]
    fn native_latex_bmod_computes_the_remainder() {
        // `a \bmod b` with a=7, b=3 is 7 mod 3 = 1, computed on the native
        // ComputeOp::Mod via the operator-name-juxtaposition path: `\bmod` has no
        // operator-table entry, so it lowers to Symbol("bmod") inside the implicit
        // multiplication `Bin(Mul, Bin(Mul, a, bmod), b)`, which the adapter
        // recognises as `a mod b` — NOT the bare product `a * bmod * b`.
        let d = crate::compile_and_decide(
            "observe a(7)\n\
             observe b(3)\n\
             let answer = latex \"$a \\bmod b$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 1 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(d.ranked[0].posterior > 0.99, "{d:?}");
    }

    #[test]
    fn native_latex_pmod_matches_bmod_and_carries_the_dividend_sign() {
        // `a \pmod{b}` computes the same remainder as `\bmod` (a=7, b=3 → 1); and a
        // negative dividend keeps its sign — `(0 − a) \bmod b` = −7 mod 3 = −1 (Rust
        // `%`), the whole point of ComputeOp::Mod versus a Euclidean/floored modulo
        // (which would give +2).
        let p = crate::compile_and_decide(
            "observe a(7)\n\
             observe b(3)\n\
             let answer = latex \"$a \\pmod{b}$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 1 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(p.ranked[0].posterior > 0.99, "{p:?}");
        let neg = crate::compile_and_decide(
            "observe a(7)\n\
             observe b(3)\n\
             let answer = latex \"$(0 - a) \\bmod b$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == -1 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(neg.ranked[0].posterior > 0.99, "{neg:?}");
    }

    #[test]
    fn native_latex_sgn_computes_the_sign() {
        // `\operatorname{sgn}(x)` with x=5 is +1; with x=−5 (written 0−x) is −1 — the
        // native ComputeOp::Sign via the operator-name-juxtaposition path
        // (`Bin(Mul, Text("sgn"), (x))`), the same shape as `\operatorname{trunc}`.
        let pos = crate::compile_and_decide(
            "observe x(5)\n\
             let answer = latex \"$\\operatorname{sgn}(x)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 1 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(pos.ranked[0].posterior > 0.99, "{pos:?}");
        let neg = crate::compile_and_decide(
            "observe x(5)\n\
             let answer = latex \"$\\operatorname{sgn}(0 - x)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == -1 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(neg.ranked[0].posterior > 0.99, "{neg:?}");
    }

    #[test]
    fn native_latex_sgn_of_a_net_difference_gives_the_direction() {
        // `\operatorname{sgn}(a - b)` with a=3, b=8 is sgn(−5) = −1 — the sign of a net
        // quantity, computed as a single node. Confirms the adapter passes the inner
        // difference through to ComputeOp::Sign (which returns a dimensionless ±1).
        let d = crate::compile_and_decide(
            "observe a(3)\n\
             observe b(8)\n\
             let answer = latex \"$\\operatorname{sgn}(a - b)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == -1 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(d.ranked[0].posterior > 0.99, "{d:?}");
    }

    #[test]
    fn native_latex_operatorname_floor_ceil_round_word_spellings() {
        // The word-spelled roundings reach the SAME ComputeOps as their Unicode-bracket
        // twins (`⌊⌋`/`⌈⌉`/`⌊⌉`). `\operatorname{…}` is a TEXT command, so each parses as
        // the operator-name juxtaposition `Bin(Mul, Text(name), (arg))` — the same shape as
        // `\operatorname{trunc}`/`\operatorname{sgn}` — which the adapter now intercepts.
        // Values chosen so the answer is unambiguous under any half-rounding convention:
        //   floor(7/2) = floor(3.5) = 3,  ceil(7/2) = ceil(3.5) = 4,  round(9/4) = round(2.25) = 2.
        let floor = crate::compile_and_decide(
            "observe a(7)\n\
             observe b(2)\n\
             let answer = latex \"$\\operatorname{floor}(a / b)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 3 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(floor.ranked[0].posterior > 0.99, "{floor:?}");
        let ceil = crate::compile_and_decide(
            "observe a(7)\n\
             observe b(2)\n\
             let answer = latex \"$\\operatorname{ceil}(a / b)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 4 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(ceil.ranked[0].posterior > 0.99, "{ceil:?}");
        let round = crate::compile_and_decide(
            "observe a(9)\n\
             observe b(4)\n\
             let answer = latex \"$\\operatorname{round}(a / b)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 2 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(round.ranked[0].posterior > 0.99, "{round:?}");
    }

    #[test]
    fn native_latex_exp_of_ln_round_trips() {
        // `\exp(\ln(x))` with x=5 returns 5, computed on the native transcendental
        // ComputeOp::Exp/Ln via the `MathExpr::Call` → `ExprAst::Call` path. A
        // dimensionless observed value is required (a transcendental of a pure
        // number). The contribution fires only if the round-trip lands on 5.
        let d = crate::compile_and_decide(
            "observe x(5)\n\
             let answer = latex \"$\\exp(\\ln(x))$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 5 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(d.ranked[0].posterior > 0.99, "{d:?}");
    }

    #[test]
    fn native_latex_cos_of_zero_is_one() {
        // `\cos(x)` with x=0 is 1 — a π-free anchor that the named-function call
        // lowers and the engine computes on ComputeOp::Cos.
        let d = crate::compile_and_decide(
            "observe x(0)\n\
             let answer = latex \"$\\cos(x)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 1 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(d.ranked[0].posterior > 0.99, "{d:?}");
    }

    #[test]
    fn native_latex_extended_trig_family_lowers() {
        // The rest of the trig family the latex frontend parses — hyperbolic
        // `\cosh(x)` (cosh 0 = 1) and inverse `\arctan(x)` (atan 0 = 0) — now lower
        // to the native ComputeOp::Cosh/Atan instead of erroring as unsupported.
        let cosh = crate::compile_and_decide(
            "observe x(0)\n\
             let answer = latex \"$\\cosh(x)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 1 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(cosh.ranked[0].posterior > 0.99, "{cosh:?}");
        let atan = crate::compile_and_decide(
            "observe x(0)\n\
             let answer = latex \"$\\arctan(x)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 0 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(atan.ranked[0].posterior > 0.99, "{atan:?}");
    }

    #[test]
    fn native_latex_binary_min_max_lower_to_native_ops() {
        // `\min(a, b)` / `\max(a, b)` — the first BINARY-Call lowering. The latex
        // frontend parses the argument as a two-element Sequence; the adapter now
        // lowers it to ComputeOp::Min2/Max2 instead of erroring as unsupported.
        let mn = crate::compile_and_decide(
            "observe a(3)\n\
             observe b(8)\n\
             let answer = latex \"$\\min(a, b)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 3 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(mn.ranked[0].posterior > 0.99, "{mn:?}");
        let mx = crate::compile_and_decide(
            "observe a(3)\n\
             observe b(8)\n\
             let answer = latex \"$\\max(a, b)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 8 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(mx.ranked[0].posterior > 0.99, "{mx:?}");
    }

    #[test]
    fn native_latex_min_with_one_argument_is_rejected() {
        // `\min(a)` (a single argument) has no fold — min/max/gcd/lcm need TWO or
        // more operands — so it is a clean, explicit error rather than a silent
        // mis-lowering. (Three-or-more args ARE now accepted; see the n-ary test.)
        let src = "observe a(3)\nlet answer = latex \"$\\min(a)$\"\n? answer\n";
        assert!(
            compile(src).is_err(),
            "one-arg min must be rejected: {src:?}"
        );
    }

    #[test]
    fn native_latex_nary_min_max_left_fold_over_three_or_more_args() {
        // `\min`/`\max`/`\gcd`/`\lcm` accept TWO OR MORE comma-separated operands and
        // left-fold into a chain of the associative binary op — min(a, b, c) =
        // min(min(a, b), c) — reusing ComputeOp::Min2/Max2/Gcd/Lcm, no n-ary engine op.
        // min(7, 3, 9, 2) = 2; max(7, 3, 9, 2) = 9.
        let mn = crate::compile_and_decide(
            "observe a(7)\n\
             observe b(3)\n\
             observe c(9)\n\
             observe d(2)\n\
             let answer = latex \"$\\min(a, b, c, d)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 2 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(mn.ranked[0].posterior > 0.99, "{mn:?}");
        let mx = crate::compile_and_decide(
            "observe a(7)\n\
             observe b(3)\n\
             observe c(9)\n\
             observe d(2)\n\
             let answer = latex \"$\\max(a, b, c, d)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 9 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(mx.ranked[0].posterior > 0.99, "{mx:?}");
        // gcd(24, 36, 60) = 12 (three-arg gcd fold); lcm(2, 3, 4) = 12.
        let g = crate::compile_and_decide(
            "observe a(24)\n\
             observe b(36)\n\
             observe c(60)\n\
             let answer = latex \"$\\gcd(a, b, c)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 12 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(g.ranked[0].posterior > 0.99, "{g:?}");
        let l = crate::compile_and_decide(
            "observe a(2)\n\
             observe b(3)\n\
             observe c(4)\n\
             let answer = latex \"$\\lcm(a, b, c)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 12 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(l.ranked[0].posterior > 0.99, "{l:?}");
    }

    #[test]
    fn native_latex_binary_gcd_lcm_lower_to_native_ops() {
        // `\gcd(a, b)` / `\lcm(a, b)` reuse the binary-Call path (Call2) and lower
        // to ComputeOp::Gcd/Lcm. gcd(12, 18) = 6; lcm(4, 6) = 12.
        let g = crate::compile_and_decide(
            "observe a(12)\n\
             observe b(18)\n\
             let answer = latex \"$\\gcd(a, b)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 6 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(g.ranked[0].posterior > 0.99, "{g:?}");
        let l = crate::compile_and_decide(
            "observe a(4)\n\
             observe b(6)\n\
             let answer = latex \"$\\lcm(a, b)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 12 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(l.ranked[0].posterior > 0.99, "{l:?}");
    }

    #[test]
    fn native_latex_operatorname_nary_word_spellings() {
        // `\operatorname{min}(a, b)` / `\operatorname{max}` / `\operatorname{gcd}` /
        // `\operatorname{lcm}` — the operator-name spellings of the variadic binary
        // functions. `\operatorname{…}` is a TEXT command, so these parse as the
        // juxtaposition `Bin(Mul, Text("gcd"), (a, b))` rather than a `Call`; the adapter
        // recognises that shape and folds through the SAME `Call2` chain as the `\gcd(…)`
        // spelling, so a model that writes the operator name reaches the identical native
        // op. Two-arg: min(3, 8) = 3; max(3, 8) = 8.
        let mn = crate::compile_and_decide(
            "observe a(3)\n\
             observe b(8)\n\
             let answer = latex \"$\\operatorname{min}(a, b)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 3 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(mn.ranked[0].posterior > 0.99, "{mn:?}");
        let mx = crate::compile_and_decide(
            "observe a(3)\n\
             observe b(8)\n\
             let answer = latex \"$\\operatorname{max}(a, b)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 8 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(mx.ranked[0].posterior > 0.99, "{mx:?}");
        // Two-arg gcd/lcm: gcd(12, 18) = 6; lcm(4, 6) = 12.
        let g = crate::compile_and_decide(
            "observe a(12)\n\
             observe b(18)\n\
             let answer = latex \"$\\operatorname{gcd}(a, b)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 6 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(g.ranked[0].posterior > 0.99, "{g:?}");
        // Three-or-more args left-fold identically to the `\gcd(…)` spelling:
        // lcm(2, 3, 4) = 12.
        let l = crate::compile_and_decide(
            "observe a(2)\n\
             observe b(3)\n\
             observe c(4)\n\
             let answer = latex \"$\\operatorname{lcm}(a, b, c)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 12 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(l.ranked[0].posterior > 0.99, "{l:?}");
    }

    #[test]
    fn native_latex_operatorname_min_with_one_argument_is_rejected() {
        // `\operatorname{min}(a)` — a single argument has no fold (min/max/gcd/lcm need
        // TWO or more operands), so it is a clean, explicit error via the SAME
        // `latex_nary_fold` guard the `\min(a)` spelling hits — never a silent
        // mis-lowering into a bare product.
        let src = "observe a(3)\nlet answer = latex \"$\\operatorname{min}(a)$\"\n? answer\n";
        assert!(
            compile(src).is_err(),
            "one-arg operatorname min must be rejected: {src:?}"
        );
    }

    #[test]
    fn native_latex_operatorname_unary_word_spellings() {
        // `\operatorname{abs}(x)` / `\operatorname{exp}(x)` / `\operatorname{log}(x)` /
        // `\operatorname{ln}(x)` — the operator-name spellings of single-argument unary
        // functions that already have a native op. `\operatorname{…}` is a TEXT command, so
        // these parse as the juxtaposition `Bin(Mul, Text("exp"), (x))` rather than a `Call`
        // (`\exp`) or a `Fenced` (`|x|`); the adapter recognises that shape and lowers to the
        // SAME node — `abs`→ExprAst::Abs, `exp`/`log`/`ln`→ExprAst::Call(NamedFn::…).
        // abs(a - b) with a=3, b=10 → 7 (dimension-preserving magnitude).
        let a = crate::compile_and_decide(
            "observe a(3)\n\
             observe b(10)\n\
             let answer = latex \"$\\operatorname{abs}(a - b)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 7 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(a.ranked[0].posterior > 0.99, "{a:?}");
        // exp(ln(x)) round-trips to x — exercises BOTH the exp and ln operator-name arms in
        // one expression. x = 5 → 5.
        let e = crate::compile_and_decide(
            "observe x(5)\n\
             let answer = latex \"$\\operatorname{exp}(\\operatorname{ln}(x))$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 5 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(e.ranked[0].posterior > 0.99, "{e:?}");
        // log is base-10 (distinct from ln): log(1000) = 3.
        let l = crate::compile_and_decide(
            "observe x(1000)\n\
             let answer = latex \"$\\operatorname{log}(x)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 3 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(l.ranked[0].posterior > 0.99, "{l:?}");
    }

    #[test]
    fn native_latex_operatorname_trig_family_word_spellings() {
        // `\operatorname{sin}(x)` / `\operatorname{cos}` / `\operatorname{arctan}` / … — the
        // operator-name spellings of the whole trig family. `\operatorname{…}` is a TEXT command,
        // so these parse as the juxtaposition `Bin(Mul, Text("sin"), (x))` rather than a `Call`
        // (`\sin`); the adapter recognises that shape via `operator_name_trig_fn` and lowers to the
        // SAME `ExprAst::Call(NamedFn::…)` the macro produces. Transcendental ⇒ Scalar→Scalar.
        // cos(x) with x = 0 → 1.
        let c = crate::compile_and_decide(
            "observe x(0)\n\
             let answer = latex \"$\\operatorname{cos}(x)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 1 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(c.ranked[0].posterior > 0.99, "{c:?}");
        // sinh(x) with x = 0 → 0 (hyperbolic).
        let sh = crate::compile_and_decide(
            "observe x(0)\n\
             let answer = latex \"$\\operatorname{sinh}(x)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 0 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(sh.ranked[0].posterior > 0.99, "{sh:?}");
        // The `arc…` alias resolves to the same inverse function: arctan(x) with x = 0 → 0.
        let at = crate::compile_and_decide(
            "observe x(0)\n\
             let answer = latex \"$\\operatorname{arctan}(x)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 0 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(at.ranked[0].posterior > 0.99, "{at:?}");
    }

    #[test]
    fn native_latex_reciprocal_hyperbolic_functions_lower() {
        // `\coth`/`\sech`/`\csch` — the reciprocal hyperbolics. None is a frontend `Func`, so each
        // arrives as the operator-name juxtaposition `Bin(Mul, Symbol("coth"), (x))` (bare macro).
        // The adapter composes `1 / f(x)` from the hyperbolic NamedFn it is the reciprocal of:
        // coth = 1/tanh, sech = 1/cosh, csch = 1/sinh. Exact anchor: sech(0) = 1/cosh(0) = 1/1 = 1.
        let sech0 = crate::compile_and_decide(
            "observe x(0)\n\
             let answer = latex \"$\\sech(x)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 1 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(sech0.ranked[0].posterior > 0.99, "{sech0:?}");
        // coth(1) = 1/tanh(1) ≈ 1.3130352854993315 (matched within the engine's 1e-9 == tolerance).
        let coth1 = crate::compile_and_decide(
            "observe x(1)\n\
             let answer = latex \"$\\coth(x)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 1.3130352854993315 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(coth1.ranked[0].posterior > 0.99, "{coth1:?}");
        // csch(1) = 1/sinh(1) ≈ 0.8509181282393216.
        let csch1 = crate::compile_and_decide(
            "observe x(1)\n\
             let answer = latex \"$\\csch(x)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 0.8509181282393216 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(csch1.ranked[0].posterior > 0.99, "{csch1:?}");
        // The `\operatorname{sech}(x)` word spelling reaches the SAME composition (Text, not Symbol):
        // sech(1) = 1/cosh(1) ≈ 0.6480542736638855.
        let opsech = crate::compile_and_decide(
            "observe x(1)\n\
             let answer = latex \"$\\operatorname{sech}(x)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 0.6480542736638855 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(opsech.ranked[0].posterior > 0.99, "{opsech:?}");
    }

    #[test]
    fn native_latex_inverse_hyperbolic_functions_lower() {
        // `\operatorname{arsinh}`/`\operatorname{arcosh}`/`\operatorname{artanh}` — the inverse
        // (area) hyperbolics. None is a frontend `Func`, so each arrives as the operator-name
        // juxtaposition `Bin(Mul, Text("arsinh"), (x))` (or `Bin(Mul, Symbol("arsinh"), (x))` for the
        // bare macro). The adapter composes each from its logarithm identity using only `ln` + `^`,
        // so the results are the standard real branch. Exact anchor: arsinh(0) = ln(0 + √1) = 0.
        let arsinh0 = crate::compile_and_decide(
            "observe x(0)\n\
             let answer = latex \"$\\operatorname{arsinh}(x)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 0 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(arsinh0.ranked[0].posterior > 0.99, "{arsinh0:?}");
        // arsinh(1) = ln(1 + √2) ≈ 0.8813735870195429 (matched within the engine's 1e-9 == tolerance).
        let arsinh1 = crate::compile_and_decide(
            "observe x(1)\n\
             let answer = latex \"$\\operatorname{arsinh}(x)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 0.8813735870195429 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(arsinh1.ranked[0].posterior > 0.99, "{arsinh1:?}");
        // arcosh(2) = ln(2 + √3) ≈ 1.3169578969248166.
        let arcosh2 = crate::compile_and_decide(
            "observe x(2)\n\
             let answer = latex \"$\\operatorname{arcosh}(x)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 1.3169578969248166 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(arcosh2.ranked[0].posterior > 0.99, "{arcosh2:?}");
        // artanh(0.5) = 0.5·ln(1.5/0.5) = 0.5·ln 3 ≈ 0.5493061443340549. The bare `\artanh` macro
        // reaches the SAME composition through a `Symbol` (not `Text`) name.
        let artanh = crate::compile_and_decide(
            "observe x(0.5)\n\
             let answer = latex \"$\\artanh(x)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 0.5493061443340549 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(artanh.ranked[0].posterior > 0.99, "{artanh:?}");
        // The terse `asinh` spelling reaches the SAME `arsinh` composition: asinh(1) ≈ 0.8813735870195429.
        let asinh1 = crate::compile_and_decide(
            "observe x(1)\n\
             let answer = latex \"$\\operatorname{asinh}(x)$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 0.8813735870195429 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(asinh1.ranked[0].posterior > 0.99, "{asinh1:?}");
    }

    #[test]
    fn native_latex_accent_wrapped_operands_are_transparent() {
        // `\hat{x}` / `\bar{x}` / … — an accent is a notational decoration, not an operation. A
        // model that writes a statistics formula (`\hat{p}(1-\hat{p})`, `\bar{x} - \bar{y}`) means
        // the accented symbol to carry its operand's value; the adapter lowers `Accent` transparently
        // to its inner body. `\hat{a}(b - \hat{a})` with a=3, b=10 → 3*(10-3) = 21 (and the SAME `a`
        // appears accented twice, confirming the decoration is stripped everywhere).
        let h = crate::compile_and_decide(
            "observe a(3)\n\
             observe b(10)\n\
             let answer = latex \"$\\hat{a}(b - \\hat{a})$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 21 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(h.ranked[0].posterior > 0.99, "{h:?}");
        // A different accent (`\bar`) over each operand of a sum: bar(x) + bar(y) with x=4, y=6 → 10.
        let b = crate::compile_and_decide(
            "observe x(4)\n\
             observe y(6)\n\
             let answer = latex \"$\\bar{x} + \\bar{y}$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 10 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(b.ranked[0].posterior > 0.99, "{b:?}");
    }

    #[test]
    fn native_latex_overset_underset_are_transparent_to_base() {
        // `\overset{note}{base}` / `\underset{note}{base}` (and `\overbrace`/`\underbrace`) — an
        // over/under annotation is a notational decoration, not an operation; the value is the
        // base's. The adapter lowers Overset/Underset transparently to `base`, dropping the mark.
        // `\overset{\text{sum}}{a + b}` with a=3, b=4 → 7 (the annotation is discarded).
        let over = crate::compile_and_decide(
            "observe a(3)\n\
             observe b(4)\n\
             let answer = latex \"$\\overset{s}{a + b}$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 7 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(over.ranked[0].posterior > 0.99, "{over:?}");
        // `\underbrace{a * b}` (an Underset with a brace mark) with a=6, b=7 → 42.
        let under = crate::compile_and_decide(
            "observe a(6)\n\
             observe b(7)\n\
             let answer = latex \"$\\underbrace{a * b}$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 42 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(under.ranked[0].posterior > 0.99, "{under:?}");
    }

    #[test]
    fn native_latex_finite_sum_and_product_unroll() {
        // `\sum_{i=1}^{3} i` = 1 + 2 + 3 = 6 (bare index body).
        let s = crate::compile_and_decide(
            "let answer = latex \"$\\sum_{i=1}^{3} i$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 6 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(s.ranked[0].posterior > 0.99, "{s:?}");
        // `\sum_{i=1}^{3} x_i` = x_1 + x_2 + x_3 (composes with subscripts): 2 + 3 + 4 = 9.
        let sx = crate::compile_and_decide(
            "observe x_1(2)\n\
             observe x_2(3)\n\
             observe x_3(4)\n\
             let answer = latex \"$\\sum_{i=1}^{3} x_i$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 9 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(sx.ranked[0].posterior > 0.99, "{sx:?}");
        // `\prod_{k=1}^{4} k` = 1 · 2 · 3 · 4 = 24.
        let p = crate::compile_and_decide(
            "let answer = latex \"$\\prod_{k=1}^{4} k$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 24 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(p.ranked[0].posterior > 0.99, "{p:?}");
    }

    #[test]
    fn native_latex_symbolic_and_integral_bigops_are_rejected() {
        // A symbolic upper bound (`n`) has no concrete count — cannot unroll, must reject.
        let symbolic = compile(
            "observe n(3)\n\
             let answer = latex \"$\\sum_{i=1}^{n} i$\"\n\
             ? answer\n",
        );
        assert!(
            symbolic.is_err(),
            "symbolic-bound sum must reject: {symbolic:?}"
        );
        // A definite integral is not a finite sum/product — must reject, never approximate.
        let integral = compile(
            "observe x(5)\n\
             let answer = latex \"$\\int_0^1 x$\"\n\
             ? answer\n",
        );
        assert!(integral.is_err(), "integral must reject: {integral:?}");
    }

    #[test]
    fn native_latex_deep_sum_body_does_not_overflow() {
        // A summation BODY that is a very deep juxtaposition forms a `Bin(Mul)` spine the latex
        // parser's MAX_DEPTH does not bound. Braces make the whole juxtaposition the sum's body
        // (`\sum_{i=1}^{2} {aaaa…}`), so it routes through the depth-budgeted `substitute_index`,
        // which must REJECT it (return an error) rather than overflow the thread stack. A
        // 20,000-letter braced body: this must return instead of aborting.
        let deep_body = "a".repeat(20_000);
        let src = format!(
            "observe a(1)\n\
             let answer = latex \"$\\sum_{{i=1}}^{{2}} {{{deep_body}}}$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 1 to correct\n\
             ? correct\n"
        );
        // Either outcome is acceptable — the guarantee is that it returns instead of aborting.
        let _ = compile(&src);
    }

    #[test]
    fn native_latex_binomial_coefficient_computes() {
        // `\binom{5}{2}` = "5 choose 2" = 10 — the COUNT C(n,k), distinct from the ratio 5/2.
        let basic = crate::compile_and_decide(
            "let answer = latex \"$\\binom{5}{2}$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 10 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(basic.ranked[0].posterior > 0.99, "{basic:?}");
        // `\dbinom{6}{3}` — the display-style spelling lowers to the same Binom = 20.
        let dbinom = crate::compile_and_decide(
            "let answer = latex \"$\\dbinom{6}{3}$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 20 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(dbinom.ranked[0].posterior > 0.99, "{dbinom:?}");
        // Symmetry C(9,7) = C(9,2) = 36: the loop iterates over min(k, n-k)=2, not 7.
        let symm = crate::compile_and_decide(
            "let answer = latex \"$\\binom{9}{7}$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 36 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(symm.ranked[0].posterior > 0.99, "{symm:?}");
        // Edge: C(4,0) = 1 (the empty-subset count) — the loop runs zero steps.
        let edge = crate::compile_and_decide(
            "let answer = latex \"$\\binom{4}{0}$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 1 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(edge.ranked[0].posterior > 0.99, "{edge:?}");
        // Composes with surrounding arithmetic: \binom{4}{2} + \binom{3}{1} = 6 + 3 = 9.
        let composed = crate::compile_and_decide(
            "let answer = latex \"$\\binom{4}{2} + \\binom{3}{1}$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 9 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(composed.ranked[0].posterior > 0.99, "{composed:?}");
    }

    #[test]
    fn native_latex_undecidable_binomials_are_rejected() {
        // A symbolic argument has no concrete count — cannot evaluate, must reject (never guess n/k).
        let symbolic = compile(
            "observe n(6)\n\
             observe k(2)\n\
             let answer = latex \"$\\binom{n}{k}$\"\n\
             ? answer\n",
        );
        assert!(
            symbolic.is_err(),
            "symbolic binomial must reject: {symbolic:?}"
        );
        // k > n is out of the supported domain (\binom{3}{5}) — reject rather than emit 0.
        let inverted = compile(
            "let answer = latex \"$\\binom{3}{5}$\"\n\
             ? answer\n",
        );
        assert!(inverted.is_err(), "k>n binomial must reject: {inverted:?}");
        // A result beyond the f64 exact-integer range (C(60,30) ≈ 1.18e17) is rejected rather than
        // emitting a silently-rounded literal.
        let too_large = compile(
            "let answer = latex \"$\\binom{60}{30}$\"\n\
             ? answer\n",
        );
        assert!(
            too_large.is_err(),
            "too-large binomial must reject: {too_large:?}"
        );
        // An upper argument beyond BINOM_N_CAP is rejected before looping.
        let oversized = compile(
            "let answer = latex \"$\\binom{2000}{2}$\"\n\
             ? answer\n",
        );
        assert!(
            oversized.is_err(),
            "oversized-n binomial must reject: {oversized:?}"
        );
    }

    #[test]
    fn native_latex_deep_binomial_argument_does_not_overflow() {
        // A binomial argument that is a very deep juxtaposition forms a `Bin(Mul)` spine the latex
        // parser's MAX_DEPTH does not bound. Braces make the whole juxtaposition the binomial's
        // `n` slot (`\binom{aaaa…}{2}`). The adapter reads that slot with the NON-recursive
        // `number_as_i64`, which returns `None` on the outermost `Bin` WITHOUT descending — so the
        // binomial is rejected without ever walking the spine, and compilation must RETURN (an
        // error) rather than overflow the thread stack. A 20,000-letter braced argument.
        let deep = "a".repeat(20_000);
        let src = format!(
            "let answer = latex \"$\\binom{{{deep}}}{{2}}$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 1 to correct\n\
             ? correct\n"
        );
        // Either outcome is acceptable — the guarantee is that it returns instead of aborting.
        let _ = compile(&src);
    }

    #[test]
    fn native_latex_subscripts_bind_as_distinct_variables() {
        // `x_i` / `x_1` / `V_{max}` — a subscript names a DISTINCT variable, not a computation.
        // The adapter mangles the subscript into a flat `base_sub` identifier that binds to a
        // matching `observe`. Letter subscripts: x_i + x_j with x_i=5, x_j=8 → 13 (two distinct
        // observed quantities, NOT `x*(i+j)`).
        let letters = crate::compile_and_decide(
            "observe x_i(5)\n\
             observe x_j(8)\n\
             let answer = latex \"$x_i + x_j$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 13 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(letters.ranked[0].posterior > 0.99, "{letters:?}");
        // Braced multi-letter subscripts: c_{max} - c_{min} with c_max=100, c_min=30 → 70. The
        // `{max}` / `{min}` juxtaposition chains flatten back into the words `max`/`min`. (The base
        // is lowercase because the ADJ surface lexer — separately from this adapter — requires
        // `observe` identifiers to start lowercase; the mangled `c_max` binds to `observe c_max`.)
        let words = crate::compile_and_decide(
            "observe c_max(100)\n\
             observe c_min(30)\n\
             let answer = latex \"$c_{max} - c_{min}$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 70 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(words.ranked[0].posterior > 0.99, "{words:?}");
        // Numeric subscripts name enumerated variables: x_1 * x_2 with x_1=6, x_2=7 → 42.
        let digits = crate::compile_and_decide(
            "observe x_1(6)\n\
             observe x_2(7)\n\
             let answer = latex \"$x_1 * x_2$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 42 to correct\n\
             ? correct\n",
        )
        .unwrap();
        assert!(digits.ranked[0].posterior > 0.99, "{digits:?}");
    }

    #[test]
    fn native_latex_deep_juxtaposed_subscript_does_not_overflow() {
        // A braced subscript of thousands of juxtaposed letters (`x_{aaaa…a}`) parses to a deep
        // left-associative `Bin(Mul)` spine that the latex parser's `MAX_DEPTH` does NOT bound
        // (juxtaposition is built in a loop). The subscript flattener MUST walk that spine
        // iteratively — a naive recursion would overflow the thread stack (an uncatchable abort /
        // DoS). This program must lower without panicking: the giant `x_aaaa…` identifier simply
        // never matches an `observe`, so `answer` binds nothing and the guarded contribution stays
        // dormant — compilation completes, which is the point (no stack overflow).
        let deep = "a".repeat(20_000);
        let src = format!(
            "observe x_a(1)\n\
             let answer = latex \"$x_{{{deep}}}$\"\n\
             prior 0.10 for correct\n\
             contributes 1000000 from answer == 1 to correct\n\
             ? correct\n"
        );
        // Either outcome is acceptable — the guarantee is that it returns instead of aborting.
        let _ = compile(&src);
    }

    #[test]
    fn native_latex_symbolic_root_degree_is_rejected() {
        // `\sqrt[k]{x}` — a symbolic degree has no numeric value, so the reciprocal
        // exponent `1/k` cannot be formed. It must be rejected, not silently
        // mislowered.
        let err = compile(
            "observe x(16)\n\
             let answer = latex \"$\\sqrt[k]{x}$\"\n\
             ? answer\n",
        );
        assert!(
            err.is_err(),
            "symbolic root degree must be rejected: {err:?}"
        );
    }

    #[test]
    fn native_latex_zero_root_degree_is_rejected() {
        // `\sqrt[0]{x}` — a zero degree would make the exponent `1/0` undefined, so
        // it is rejected at adapt time (positive integer degrees only).
        let err = compile(
            "observe x(16)\n\
             let answer = latex \"$\\sqrt[0]{x}$\"\n\
             ? answer\n",
        );
        assert!(err.is_err(), "zero root degree must be rejected: {err:?}");
    }

    #[test]
    fn all_relational_operators_parse() {
        for (src, want) in [
            ("constrain a >= 1", crate::ast::RelOp::Ge),
            ("constrain a <= 1", crate::ast::RelOp::Le),
            ("constrain a > 1", crate::ast::RelOp::Gt),
            ("constrain a < 1", crate::ast::RelOp::Lt),
            ("constrain a == 1", crate::ast::RelOp::Eq),
            ("constrain a = 1", crate::ast::RelOp::Eq),
            ("constrain a != 1", crate::ast::RelOp::Ne),
        ] {
            let lowered = compile(src).unwrap();
            assert_eq!(lowered.constraints.constraints[0].op, want, "for {src:?}");
        }
    }

    #[test]
    fn a_pure_rulebook_has_an_empty_constraint_system() {
        let lowered = compile("prior 0.10 for acs\n? acs").unwrap();
        assert!(lowered.constraints.is_empty());
    }

    // ---- ADJ73 PR-C: precedence surface syntax (functional + priority tiers) ----

    /// `functional` + `priority:` lower into the engine so a higher-tier rule GOVERNS a
    /// conflicting lower-tier one (the whole point — surface syntax over the merged engine).
    #[test]
    fn functional_and_priority_tiers_resolve_a_conflict() {
        use logic_engine::{enumerate_governing, GovernStatus};
        let src = "\
functional timing(decision)
relate stable_routine_pending(yes)
rule { head: timing(await_culture) when: stable_routine_pending(yes) priority: specific }
rule { head: timing(treat_now) when: stable_routine_pending(yes) priority: default }
? timing($D)";
        let lowered = compile(src).unwrap();
        let res = enumerate_governing(&lowered.queries[0], &lowered.kb);
        let governing: Vec<&CoreTerm> = res.governing().map(|a| &a.term).collect();
        assert_eq!(
            governing.len(),
            1,
            "exactly one answer should govern: {res:?}"
        );
        // the `specific` tier governs; the `default` is defeated but retained.
        assert!(matches!(
            governing[0],
            CoreTerm::Compound { functor, args }
                if functor == "timing" && args == &[CoreTerm::Atom("await_culture".into())]
        ));
        assert!(!res.has_conflict());
        let defeated = res
            .answers
            .iter()
            .find(|a| {
                matches!(&a.term, CoreTerm::Compound { args, .. }
                if args == &[CoreTerm::Atom("treat_now".into())])
            })
            .unwrap();
        assert!(matches!(defeated.status, GovernStatus::Defeated { .. }));
    }

    /// Two equal tiers on a functional predicate → an unresolved CONFLICT (no governor).
    #[test]
    fn equal_tiers_on_a_functional_predicate_conflict() {
        use logic_engine::enumerate_governing;
        let prog = "\
functional pick(x)
relate gate(t)
rule { head: pick(a) when: gate(t) priority: authoritative }
rule { head: pick(b) when: gate(t) priority: authoritative }
? pick($X)";
        let lowered = compile(prog).unwrap();
        let res = enumerate_governing(&lowered.queries[0], &lowered.kb);
        assert!(
            res.has_conflict(),
            "equal tiers must conflict, not silently pick: {res:?}"
        );
        assert_eq!(res.governing().count(), 0);
    }

    /// A non-functional predicate is unaffected — every derivation governs (back-compat).
    #[test]
    fn priority_without_functional_leaves_every_answer_governing() {
        use logic_engine::enumerate_governing;
        let prog = "\
relate gate(t)
rule { head: note(a) when: gate(t) priority: specific }
rule { head: note(b) when: gate(t) priority: default }
? note($X)";
        let lowered = compile(prog).unwrap();
        let res = enumerate_governing(&lowered.queries[0], &lowered.kb);
        assert_eq!(res.governing().count(), 2, "non-functional → both govern");
        assert!(!res.has_conflict());
    }

    /// An unknown priority tier is a clean lowering error, not a silent default.
    #[test]
    fn unknown_priority_tier_is_rejected() {
        let err =
            compile("relate g(t)\nrule { head: h(a) when: g(t) priority: urgent }").unwrap_err();
        assert!(
            format!("{err:?}").contains("UnknownPriorityTier"),
            "expected UnknownPriorityTier, got {err:?}"
        );
    }

    // ---- ADJ73 PR-B: context precedence surface (`context:` + `context_order`) ----

    /// `context_order { higher > lower }` + `context:` on rules → the higher-context rule
    /// governs the lower-context one, and context precedence BEATS the priority tier (lex
    /// superior): the broad reading governs despite carrying the lower `default` tier.
    #[test]
    fn context_order_and_context_resolve_lex_superior() {
        use logic_engine::enumerate_governing;
        let prog = "\
functional means(term, reading)
context_order { ninth_circuit > district_court }
relate gate(t)
rule { head: means(waters, broad) when: gate(t) priority: default context: ninth_circuit }
rule { head: means(waters, narrow) when: gate(t) priority: authoritative context: district_court }
? means(waters, $R)";
        let lowered = compile(prog).unwrap();
        let res = enumerate_governing(&lowered.queries[0], &lowered.kb);
        let gov: Vec<&CoreTerm> = res.governing().map(|a| &a.term).collect();
        assert_eq!(gov.len(), 1, "exactly one governs: {res:?}");
        assert!(matches!(gov[0], CoreTerm::Compound { args, .. }
            if args == &[CoreTerm::Atom("waters".into()), CoreTerm::Atom("broad".into())]));
        assert!(!res.has_conflict());
    }

    /// A multi-edge `context_order` lowers every edge (transitive: federal > circuit > state).
    #[test]
    fn context_order_lowers_multiple_edges_transitively() {
        let prog = "\
context_order { federal > circuit, circuit > state }
relate x(t)
rule { head: r(a) when: x(t) }";
        let lowered = compile(prog).unwrap();
        // federal transitively outranks state via circuit.
        assert!(lowered.kb.context_outranks("federal", "state"));
        assert!(lowered.kb.context_outranks("federal", "circuit"));
        assert!(!lowered.kb.context_outranks("state", "federal"));
    }

    // ---- ADJ-RULE-SUBSTRATE RS-1: formula application as a sub-expression ----

    #[test]
    fn formula_calls_formula_composes_and_carries_both_cites() {
        // `ratio` applies `quotient` in its BODY (formula-calls-formula). The
        // engine expands the application on the CPU and composes the provenance
        // chain: ratio's own cite is primary; quotient's rides as a corroboration.
        let src = r#"
            formulabook f {
                formula quotient(dividend, divisor) = dividend / divisor
                    source "quotient def" locator "q-loc" trust authoritative
                formula ratio(numerator, denominator) = quotient(numerator, denominator)
                    source "ratio def" locator "r-loc" trust authoritative
            }
            observe numerator(3)
            observe denominator(4)
            ? ratio(numerator, denominator)
        "#;
        let lowered = compile(src).unwrap();
        let d = lowered
            .kb
            .derived_for("ratio")
            .expect("ratio bound a derived value");
        assert!((d.value - 0.75).abs() < 1e-9, "3/4 = 0.75, got {}", d.value);
        let prov = d.provenance.as_ref().expect("carries composed provenance");
        assert_eq!(prov.source, "ratio def", "ratio's cite is primary");
        // quotient's cite composes in as a corroboration — both are auditable.
        assert!(
            prov.corroborations
                .iter()
                .any(|c| c.source == "quotient def"),
            "quotient's cite composed as a corroboration: {:?}",
            prov.corroborations
        );
    }

    #[test]
    fn nested_application_in_arguments_expands() {
        // An application may appear in ANOTHER application's arguments:
        // `ratio(product(a, b), c)` — the engine expands inside-out. 6*? no:
        // product(2,3)=6, then ratio(6, 4) = 6/4 = 1.5.
        let src = r#"
            formulabook f {
                formula quotient(dividend, divisor) = dividend / divisor
                    source "q" trust authoritative
                formula product(factor_one, factor_two) = factor_one * factor_two
                    source "p" trust authoritative
                formula ratio(numerator, denominator) = quotient(numerator, denominator)
                    source "r" trust authoritative
            }
            observe a(2)
            observe b(3)
            observe c(4)
            let composed = ratio(product(a, b), c)
            ? composed
        "#;
        let lowered = compile(src).unwrap();
        let d = lowered.kb.derived_for("composed").unwrap();
        assert!(
            (d.value - 1.5).abs() < 1e-9,
            "(2*3)/4 = 1.5, got {}",
            d.value
        );
    }

    #[test]
    fn self_referential_formula_is_a_clean_recursion_error() {
        // `loop(x) = loop(x)` would expand forever; the depth guard turns it into a
        // clean typed error rather than a stack overflow or a hang.
        let src = r#"
            formulabook f {
                formula loop(x) = loop(x)
                    source "s" trust inferred
            }
            observe x(1)
            ? loop(x)
        "#;
        let err = compile(src).unwrap_err();
        assert!(
            matches!(
                err,
                crate::CompileError::Lower(LowerError::FormulaRecursionTooDeep { ref formula, .. })
                    if formula == "loop"
            ),
            "{err:?}"
        );
    }

    #[test]
    fn exponentially_branching_composition_trips_the_size_guard_not_the_stack() {
        // ADVERSARIAL DoS REPRO. A formula whose body reuses its parameter twice
        // (`pᵢ(x) = pᵢ₋₁(x) * pᵢ₋₁(x)`) doubles the expanded node count at every rung
        // while depth grows only LINEARLY — so the depth guard alone would let ~14
        // rungs balloon past 2¹⁴ nodes and, at higher rungs, exhaust memory. The
        // shared node budget (`FORMULA_MAX_EXPANSION_NODES`) is the real defense: it
        // bails the instant the cap is crossed, in O(cap) time, with a specific typed
        // error — NOT a hang, an OOM, or a stack overflow. (Verified out-of-band:
        // with the cap lifted, the 20-rung bomb takes ~4.6s and quadruples per added
        // rung; with the cap in place it errors in well under a millisecond.)
        //
        // 22 rungs => the fully-expanded tree would hold ~2²² leaves; the guard must
        // trip long before that. We only assert the *kind* of error, so the exact cap
        // value can move without churning this test.
        let mut src = String::from("formulabook bomb {\n");
        src.push_str("    formula p0(x) = x * x\n        source \"s\" trust inferred\n");
        for i in 1..=22 {
            src.push_str(&format!(
                "    formula p{i}(x) = p{prev}(x) * p{prev}(x)\n        source \"s\" trust inferred\n",
                prev = i - 1
            ));
        }
        src.push_str("}\nobserve x(1)\n? p22(x)\n");

        let err = compile(&src).unwrap_err();
        assert!(
            matches!(
                err,
                crate::CompileError::Lower(LowerError::FormulaExpansionTooLarge { .. })
            ),
            "exponential composition must trip the size guard, got {err:?}"
        );
    }

    #[test]
    fn deep_operator_spine_trips_the_nesting_guard_not_the_stack() {
        // ADVERSARIAL DoS REPRO #2 — the DEPTH vector the size budget cannot cover.
        // A left-leaning operator spine (`x + 0 + 0 + … + 0`) is one node wide per
        // level but arbitrarily DEEP; the expansion/substitution/clone walkers descend
        // it one native stack frame per level. With ONLY the 10 000-node size budget,
        // a deep spine builds thousands of stack frames before the budget (which trips
        // at the BOTTOM of the descent) fires — a stack overflow / SIGABRT, not a
        // catchable error. The nesting guard (`FORMULA_MAX_NODE_DEPTH`) caps the
        // lowering walkers' recursion, so the spine becomes a clean typed error.
        //
        // We build the spine AS AN AST directly (not from source) to isolate the
        // lowering guard: the upstream recursive-descent parser has its own, separate
        // deep-expression limit — a pre-existing walker this rung does not touch —
        // that would overflow the ~2 MiB test-thread stack on a spine this deep before
        // lowering ever ran. Constructing the tree with an iterative loop and calling
        // `expand_applies` directly exercises exactly the code RS-1 added, and proves
        // it returns `FormulaNestingTooDeep` on a 2 MiB stack instead of aborting.
        let mut spine = ExprAst::Ref("x".to_string());
        for _ in 0..400 {
            spine = ExprAst::Bin(ArithOp::Add, Box::new(spine), Box::new(ExprAst::Lit(0.0)));
        }
        let formulas: HashMap<&str, &FormulaDef> = HashMap::new();
        let result = expand_applies(&spine, &formulas, 0);
        assert!(
            matches!(result, Err(LowerError::FormulaNestingTooDeep { limit }) if limit == FORMULA_MAX_NODE_DEPTH),
            "a deep operator spine must trip the nesting guard, got {result:?}"
        );
    }

    #[test]
    fn unknown_formula_application_is_a_clean_error() {
        // An `IDENT(args)` whose callee is no registered formula is a specific,
        // typed error — distinct from an aggregation or a built-in call.
        let src = r#"
            formulabook f {
                formula a(x) = nope(x)
                    source "s" trust inferred
            }
            observe x(1)
            ? a(x)
        "#;
        let err = compile(src).unwrap_err();
        assert!(
            matches!(
                err,
                crate::CompileError::Lower(LowerError::FormulaUnknown { ref name })
                    if name == "nope"
            ),
            "{err:?}"
        );
    }

    #[test]
    fn arity_mismatch_in_application_is_a_clean_error() {
        // Applying a 2-parameter formula to 1 argument is a clean arity error.
        let src = r#"
            formulabook f {
                formula quotient(dividend, divisor) = dividend / divisor
                    source "s" trust authoritative
                formula bad(x) = quotient(x)
                    source "s" trust inferred
            }
            observe x(1)
            ? bad(x)
        "#;
        let err = compile(src).unwrap_err();
        assert!(
            matches!(
                err,
                crate::CompileError::Lower(LowerError::FormulaArity {
                    ref formula, expected: 2, got: 1
                }) if formula == "quotient"
            ),
            "{err:?}"
        );
    }

    #[test]
    fn rulebook_branches_on_a_formula_and_fires() {
        // `contributes … from bmi(body_mass, height) >= 30 to obese` — a rulebook
        // BRANCHING ON A FORMULA. The engine computes BMI on the CPU, gates the
        // saturating LR on it, and the verdict fires for a high-BMI case.
        let src = r#"
            dictionary v { define obese : hypothesis }
            formulabook m {
                formula bmi(body_mass, height) = body_mass / (height * height)
                    source "WHO BMI definition." trust authoritative
            }
            rulebook classify {
                use v
                prior 0.1 for obese trust inferred
                contributes 1000000 from bmi(body_mass, height) >= 30 to obese
                    source "WHO obesity threshold." trust authoritative
            }
            observe body_mass(100)
            observe height(1.7)
            ? obese
        "#;
        let lowered = compile(src).unwrap();
        // The BMI was computed into a derived slot the predicate gates on.
        let bmi = lowered
            .kb
            .derived_for("bmi")
            .expect("bmi computed for the branch");
        assert!(
            (bmi.value - 34.602).abs() < 0.01,
            "bmi ≈ 34.6, got {}",
            bmi.value
        );
        // The query saturates: the branch fired.
        let query = &lowered.queries[0];
        match search(query, &lowered.kb, SearchMode::LRAggregate) {
            SearchResult::LRAggregateResult { posterior, .. } => {
                assert!(
                    posterior > 0.9999,
                    "obese fires (saturates), got {posterior}"
                );
            }
            other => panic!("expected LRAggregateResult, got {other:?}"),
        }
    }

    #[test]
    fn rulebook_branch_on_formula_stays_at_prior_below_threshold() {
        // The mirror: a sub-threshold BMI does NOT fire the branch; the posterior
        // stays at the prior. Proves the branch is a real CPU comparison, not an
        // always-on contribution.
        let src = r#"
            dictionary v { define obese : hypothesis }
            formulabook m {
                formula bmi(body_mass, height) = body_mass / (height * height)
                    source "WHO BMI definition." trust authoritative
            }
            rulebook classify {
                use v
                prior 0.1 for obese trust inferred
                contributes 1000000 from bmi(body_mass, height) >= 30 to obese
                    source "WHO obesity threshold." trust authoritative
            }
            observe body_mass(65)
            observe height(1.75)
            ? obese
        "#;
        let lowered = compile(src).unwrap();
        let query = &lowered.queries[0];
        match search(query, &lowered.kb, SearchMode::LRAggregate) {
            SearchResult::LRAggregateResult { posterior, .. } => {
                assert!(
                    (posterior - 0.1).abs() < 1e-6,
                    "obese stays at the 0.1 prior (branch did not fire), got {posterior}"
                );
            }
            other => panic!("expected LRAggregateResult, got {other:?}"),
        }
    }
}
