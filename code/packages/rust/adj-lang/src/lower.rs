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
    compute, BodyLiteral, CmpOp as EngineCmpOp, ComputeExpr, ComputeOp, ContributionClause, Fact,
    JointContributionClause, KbError, KnowledgeBase, PredicateContributionClause, PriorClause,
    Priority, Provenance, Rule, TrustTier, UncertaintyMarker,
};

use std::collections::HashMap;

use crate::ast::{
    AggOp, Annotation, ArithOp, CmpOp, Define, DefineKind, Evidence, ExprAst, OptDir, Program,
    RelOp, Statement, Term as AstTerm, TrustTierName,
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
                    Evidence::Predicate { slot, op, value } => {
                        let clause = PredicateContributionClause::from_lr(
                            lower_term(conclusion),
                            slot.clone(),
                            lower_cmp_op(*op),
                            *value,
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
                // Lower with a per-query variable scope so repeated `$Var`s in one
                // goal share identity (Prolog clause-scope semantics). A ground
                // hypothesis query lowers identically to before (no variables).
                let mut vars = HashMap::new();
                queries.push(lower_term_scoped(conclusion, &mut vars));
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
            enforce_vocabulary(top_clauses, defs)?;
        }
        // Each rulebook is its own scope (its `use`, else the top-level `use`).
        for s in &program.statements {
            if let Statement::Rulebook { statements, .. } = s {
                if let Some(d) = first_use(statements).or(top_use) {
                    let defs = resolve(d)?;
                    let mut clauses: Vec<&Statement> = Vec::new();
                    flatten_clauses(statements, &mut clauses)?;
                    enforce_vocabulary(clauses.into_iter(), defs)?;
                }
            }
        }
    } else if !dictionary.is_empty() {
        enforce_vocabulary(flat.iter().copied(), &dictionary)?;
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
        ExprAst::Agg(op, slot) => ComputeExpr::Agg(lower_agg_op(*op), slot.clone()),
    }
}

fn lower_arith_op(op: ArithOp) -> ComputeOp {
    match op {
        ArithOp::Add => ComputeOp::Add,
        ArithOp::Sub => ComputeOp::Sub,
        ArithOp::Mul => ComputeOp::Mul,
        ArithOp::Div => ComputeOp::Div,
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

    Ok(Provenance::new(
        source.unwrap_or_default(),
        locator,
        trust_tier,
    ))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile;
    use logic_engine::{enumerate_all, search, SearchMode, SearchResult};

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
            core_compound_money @ _ if format!("{core_compound_money:?}").contains("money")
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
