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
    atom as core_atom, compound, float as core_float, var as core_var, LogicVar, Term as CoreTerm,
};
use logic_engine::{
    compute, BodyLiteral, Citation, CmpOp as EngineCmpOp, ComputeExpr, ComputeOp,
    ContributionClause, Fact, JointContributionClause, KbError, KnowledgeBase,
    PredicateContributionClause, PriorClause, Priority, Provenance, Rule, TrustTier,
    UncertaintyMarker,
};

use std::collections::HashMap;

use crate::ast::{
    AggOp, Annotation, ArithOp, BinFn, CmpOp, Define, DefineKind, Evidence, ExprAst, FormulaDef,
    NamedFn, OptDir, Program, RelOp, Statement, Term as AstTerm, TrustTierName,
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
    /// An `import "<path>"` survived to lowering (MYCIN-2026 M3). Imports must be
    /// resolved by [`crate::resolve`] *before* `lower` runs — reaching here means
    /// the caller used `compile` directly on a program that still has imports,
    /// instead of the import-resolving entry point. Rejected so an `import` is
    /// never silently dropped.
    UnresolvedImport {
        path: String,
    },
}

/// The result of lowering — a populated KB, any queries to run, and the
/// (possibly empty) constraint system the program declared.
#[derive(Debug)]
pub struct LoweredProgram {
    pub kb: KnowledgeBase,
    pub queries: Vec<CoreTerm>,
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
        if let Statement::Formulabook {
            formulas: defs, ..
        } = stmt
        {
            for fd in defs {
                validate_formula(fd)?;
                formulas.insert(fd.name.as_str(), fd);
            }
        }
    }

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
                    Evidence::Predicate { slot, op, rhs } => {
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
                    let cexpr = lower_expr(&substituted);
                    let derived =
                        compute(fd.name.clone(), &cexpr, &kb).map_err(|e| {
                            LowerError::ComputationFailed {
                                name: fd.name.clone(),
                                detail: format!("{e:?}"),
                            }
                        })?;
                    // The provenance-required lint already guaranteed a non-empty
                    // source at registration; stamp the resolved envelope onto the
                    // value so the derivation carries the formula's cites + trust.
                    let prov = annotations_to_provenance(&fd.annotations)?;
                    kb.add_derived(derived.with_provenance(prov));
                } else {
                    // An ordinary query. Lower with a per-query variable scope so
                    // repeated `$Var`s in one goal share identity (Prolog clause-scope
                    // semantics). A ground hypothesis query lowers as before.
                    let mut vars = HashMap::new();
                    queries.push(lower_term_scoped(conclusion, &mut vars));
                }
            }
            Statement::Let { name, expr } => {
                // Evaluate the formula against the facts (and any earlier
                // `let`s) seen so far — statements lower in source order, so a
                // `let` sees every `observe` above it. The engine builds the
                // derivation tree; the model never computed anything.
                let cexpr = lower_expr(expr);
                let derived = compute(name.clone(), &cexpr, &kb).map_err(|e| {
                    LowerError::ComputationFailed {
                        name: name.clone(),
                        detail: format!("{e:?}"),
                    }
                })?;
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
            // ---- import (MYCIN-2026 M3) ----
            // Imports are resolved away by `crate::resolve` before lowering; one
            // reaching here means `compile` was called on an unresolved program.
            Statement::Import(path) => {
                return Err(LowerError::UnresolvedImport { path: path.clone() })
            }
        }
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

fn lower_term(t: &AstTerm) -> CoreTerm {
    match t {
        AstTerm::Atom(name) => core_atom(name),
        AstTerm::Num(x) => core_float(*x),
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
        AstTerm::Num(x) => core_float(*x),
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
        ExprAst::Call2(f, a, b) => ComputeExpr::Bin(
            lower_bin_fn(*f),
            Box::new(lower_expr(a)),
            Box::new(lower_expr(b)),
        ),
        ExprAst::Agg(op, slot) => ComputeExpr::Agg(lower_agg_op(*op), slot.clone()),
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

fn annotations_to_provenance(annotations: &[Annotation]) -> Result<Provenance, LowerError> {
    let mut source: Option<String> = None;
    let mut locator: Option<String> = None;
    let mut trust: Option<TrustTier> = None;
    // ADJ-A9: corroborating citations are REPEATABLE — accumulate in source
    // order rather than rejecting duplicates.
    let mut corroborations: Vec<Citation> = Vec::new();

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
    let params: std::collections::HashSet<&str> =
        fd.params.iter().map(String::as_str).collect();
    let mut refs = Vec::new();
    collect_refs(&fd.body, &mut refs);
    for r in refs {
        if !params.contains(r.as_str()) {
            return Err(LowerError::FormulaFreeVariable {
                formula: fd.name.clone(),
                variable: r,
            });
        }
    }
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
        | ExprAst::Trunc(a)
        | ExprAst::Sign(a)
        | ExprAst::Call(_, a) => collect_refs(a, out),
    }
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
            AstTerm::Num(x) => ExprAst::Lit(*x),
            _ => {
                return Err(LowerError::FormulaBadArgument {
                    formula: fd.name.clone(),
                })
            }
        };
        subst.insert(param.clone(), bound);
    }
    Ok(substitute_expr(&fd.body, &subst))
}

/// Substitute parameter references in a formula body with their bound argument
/// expressions. A [`ExprAst::Ref`] naming a parameter becomes the bound
/// expression; a non-parameter identifier is left as-is (validation already
/// proved every identifier is a parameter, so this branch is defensive). An
/// [`ExprAst::Agg`] slot naming a parameter is rewritten to the bound slot name
/// when the binding is itself a slot reference (an aggregation folds a named
/// slot, so only a `Ref` binding is meaningful there).
fn substitute_expr(expr: &ExprAst, subst: &HashMap<String, ExprAst>) -> ExprAst {
    match expr {
        ExprAst::Ref(name) => subst.get(name).cloned().unwrap_or_else(|| expr.clone()),
        ExprAst::Lit(_) => expr.clone(),
        ExprAst::Agg(op, slot) => match subst.get(slot) {
            Some(ExprAst::Ref(bound)) => ExprAst::Agg(*op, bound.clone()),
            _ => expr.clone(),
        },
        ExprAst::Bin(op, a, b) => ExprAst::Bin(
            *op,
            Box::new(substitute_expr(a, subst)),
            Box::new(substitute_expr(b, subst)),
        ),
        ExprAst::Call2(f, a, b) => ExprAst::Call2(
            *f,
            Box::new(substitute_expr(a, subst)),
            Box::new(substitute_expr(b, subst)),
        ),
        ExprAst::Abs(a) => ExprAst::Abs(Box::new(substitute_expr(a, subst))),
        ExprAst::Floor(a) => ExprAst::Floor(Box::new(substitute_expr(a, subst))),
        ExprAst::Ceil(a) => ExprAst::Ceil(Box::new(substitute_expr(a, subst))),
        ExprAst::Round(a) => ExprAst::Round(Box::new(substitute_expr(a, subst))),
        ExprAst::Trunc(a) => ExprAst::Trunc(Box::new(substitute_expr(a, subst))),
        ExprAst::Sign(a) => ExprAst::Sign(Box::new(substitute_expr(a, subst))),
        ExprAst::Call(f, a) => ExprAst::Call(*f, Box::new(substitute_expr(a, subst))),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile;
    use logic_engine::{enumerate_all, search, SearchMode, SearchResult};

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
        assert!(lowered.queries.is_empty(), "formula query is not a hypothesis");
        let d = lowered
            .kb
            .derived_for("bmi")
            .expect("the formula bound a derived `bmi`");
        assert!(
            (d.value - 22.857).abs() < 0.01,
            "expected ≈22.857, got {}",
            d.value
        );
        let prov = d.provenance.as_ref().expect("derivation carries provenance");
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
        assert!((d.value - 20.0).abs() < 1e-9, "80 / 2² = 20, got {}", d.value);
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
        assert!((d.value - 6.0).abs() < 1e-9, "(3+6+9)/3 = 6, got {}", d.value);
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
        assert_eq!(f.params, vec!["body_mass".to_string(), "height".to_string()]);
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

        let bmi_lib = include_str!(
            "../../../../specs/data/adj-formula-stdlib/clinical/bmi.adj"
        );
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
        let d = lowered.kb.derived_for("bmi").expect("applied imported formula");
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
    fn non_finite_number_literal_is_rejected_at_parse() {
        // 1e400 overflows f64 to +inf; reject it rather than flow inf on.
        let err = compile("observe gross_income(1e400)").unwrap_err();
        assert!(
            matches!(err, crate::CompileError::Adapt(_)),
            "expected an adapter BadToken error, got {err:?}"
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
        assert!(compile(src).is_err(), "one-arg min must be rejected: {src:?}");
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
        let src =
            "observe a(3)\nlet answer = latex \"$\\operatorname{min}(a)$\"\n? answer\n";
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
        assert!(symbolic.is_err(), "symbolic-bound sum must reject: {symbolic:?}");
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
        assert!(symbolic.is_err(), "symbolic binomial must reject: {symbolic:?}");
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
        assert!(too_large.is_err(), "too-large binomial must reject: {too_large:?}");
        // An upper argument beyond BINOM_N_CAP is rejected before looping.
        let oversized = compile(
            "let answer = latex \"$\\binom{2000}{2}$\"\n\
             ? answer\n",
        );
        assert!(oversized.is_err(), "oversized-n binomial must reject: {oversized:?}");
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
}
