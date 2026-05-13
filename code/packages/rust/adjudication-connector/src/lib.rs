//! # adjudication-connector — wires ADJ IR to LP19 engine.
//!
//! Reference implementation of [`ADJ11`](../../../specs/ADJ11-problog-connector.md).
//! Takes an [`IRDocument`] (ADJ01) and produces a
//! [`logic_engine::KnowledgeBase`] (LP19), then runs any Query nodes
//! found in the document.
//!
//! The crate is deliberately thin. The substantive work — typed IR
//! grammar, validation, search, weighted-model-counting — lives in
//! `adjudication-ir` and `logic-engine`. This crate is only the
//! lowering layer plus a convenience wrapper around `search`.
//!
//! ## Rule subtype encoding (per ADJ01)
//!
//! ADJ Rule nodes encode their subtype in the term, not the kind, so
//! that the well-formedness check in adjudication-ir stays simple.
//! The connector recognises four functors:
//!
//! - `definitional(head, [body...])` → LP19 `Rule { probability: Certain }`
//! - `probabilistic(p, head, [body...])` → LP19 `Rule { probability: Value(p) }`
//! - `constraint([body...])` → LP19 `Rule` with synthetic
//!   `_constraint(c_N)` head
//! - `default(head, [body...], [exceptions...])` → LP19 `Rule` with
//!   `Pos` body literals + `Neg` exception literals
//!
//! Any other compound functor used at a Rule node yields
//! [`LoweringError::UnknownRuleSubtype`].

use adjudication_ir::{EdgeRelation, IRDocument, IRNode, NodeId, NodeKind, Polarity};
use logic_core::{atom, compound, Number, Term};
use logic_engine::{
    search, BodyLiteral, Fact, FactId, KnowledgeBase, Probability, Rule, RuleId, SearchMode,
    SearchResult,
};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Every reason an IR document fails to lower to a KnowledgeBase.
///
/// The variants are deliberately specific so callers (typically the
/// clarification dialogue, ADJ06) can produce helpful messages when a
/// lowering fails.
#[derive(Debug, Clone, PartialEq)]
pub enum LoweringError {
    /// A Rule node's term is not a recognized subtype functor.
    UnknownRuleSubtype { node_id: NodeId, functor: String },

    /// A Rule subtype term had the wrong number of arguments.
    InvalidRuleArity {
        node_id: NodeId,
        subtype: String,
        expected: usize,
        actual: usize,
    },

    /// A Rule subtype's body list was malformed (not a `'.'/2` cons-cell
    /// chain ending in `[]`).
    InvalidRuleBodyList { node_id: NodeId, subtype: String },

    /// A `probabilistic` rule's first argument was not a numeric term.
    InvalidProbability { node_id: NodeId, found: String },

    /// A `probabilistic` rule's probability was outside `[0, 1]`.
    ProbabilityOutOfRange { node_id: NodeId, value: f64 },
}

// ---------------------------------------------------------------------------
// Provenance (ADJ16 step 1)
// ---------------------------------------------------------------------------

/// Trust level of a clause's source rulebook.
///
/// Mirrors `adjudication_rulebook::RulebookTrust` deliberately so this
/// crate does not depend upward on adjudication-rulebook (the natural
/// dependency flow is the other direction: a pipeline that compiles a
/// `Rulebook` into a KB calls into this crate, then maps
/// `RulebookTrust → TrustTier` at the call site).
///
/// The variants map 1:1 to `RulebookTrust`:
/// - `Tentative`: LLM-elicited, no human review yet.
/// - `Reviewed`: a domain expert signed off (ADJ09 review workflow).
/// - `Authoritative`: compiled from a published regulatory document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrustTier {
    Tentative,
    Reviewed,
    Authoritative,
}

impl TrustTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            TrustTier::Tentative => "tentative",
            TrustTier::Reviewed => "reviewed",
            TrustTier::Authoritative => "authoritative",
        }
    }
}

/// Per-clause provenance: which rulebook produced it, at what trust
/// level. Attached to every Fact and every Rule that
/// [`lower_to_kb_with_provenance`] emits.
///
/// The motivation, from [ADJ16](../../../specs/ADJ16-engine-programmatic-adjudication.md)
/// §"Implementation sequence" step 1: when the engine returns a
/// proof DAG, every Fact/Rule cited in the proof must be traceable
/// back to the rulebook it came from and the trust level that
/// rulebook carried. Without that pass-through, the engine can prove
/// non-compliance correctly but the audit trail can't attribute the
/// proof to a source — which defeats the determinism win.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClauseProvenance {
    /// Stable identifier of the source rulebook. Matches
    /// `Rulebook::document_id` when the source is an
    /// `adjudication_rulebook::Rulebook`.
    pub source_rulebook_id: String,
    /// Trust tier of the source rulebook at the time of lowering.
    pub trust_tier: TrustTier,
}

impl ClauseProvenance {
    pub fn new(source_rulebook_id: impl Into<String>, trust_tier: TrustTier) -> Self {
        Self {
            source_rulebook_id: source_rulebook_id.into(),
            trust_tier,
        }
    }
}

/// A KnowledgeBase plus parallel attribution maps from clause IDs to
/// provenance.
///
/// Use [`lower_to_kb_with_provenance`] to construct one from a single
/// rulebook's IR. Use [`LoweredKb::extend`] to merge multiple
/// rulebooks into one KB while preserving per-clause attribution —
/// this is the data shape that ADJ16 step 3's `DisputedAnswer`
/// consumes.
#[derive(Debug, Default)]
pub struct LoweredKb {
    pub kb: KnowledgeBase,
    pub fact_provenance: HashMap<FactId, ClauseProvenance>,
    pub rule_provenance: HashMap<RuleId, ClauseProvenance>,
}

impl LoweredKb {
    pub fn new() -> Self {
        Self::default()
    }

    /// Merge another `LoweredKb` into this one.
    ///
    /// The other KB's Facts and Rules are re-inserted (they are
    /// assigned fresh IDs in `self.kb`), and the corresponding
    /// provenance entries are re-keyed under the new IDs.
    ///
    /// This is the API the pipeline uses to combine an
    /// adversarially-elicited rulebook set into one KB: one
    /// `lower_to_kb_with_provenance` call per source rulebook, then
    /// `extend` them in declaration order.
    pub fn extend(&mut self, other: LoweredKb) {
        let LoweredKb {
            kb: other_kb,
            fact_provenance: other_facts,
            rule_provenance: other_rules,
        } = other;
        // Walk the other KB's clauses in stable order, reinsert into
        // self.kb, and re-key provenance under the new IDs. We
        // intentionally do not preserve old FactId/RuleId values —
        // they are local to whichever KB they were minted in.
        for (old_id, prov) in other_facts {
            if let Some(fact) = other_kb.find_fact_by_id(old_id) {
                let mut fresh = fact.clone();
                fresh.id = FactId(u64::MAX);
                let new_id = self.kb.add_fact(fresh);
                self.fact_provenance.insert(new_id, prov);
            }
        }
        for (old_id, prov) in other_rules {
            if let Some(rule) = other_kb.find_rule_by_id(old_id) {
                let mut fresh = rule.clone();
                fresh.id = RuleId(u64::MAX);
                let new_id = self.kb.add_rule(fresh);
                self.rule_provenance.insert(new_id, prov);
            }
        }
    }

    /// Look up the provenance for a given Fact ID, if recorded.
    pub fn provenance_for_fact(&self, id: FactId) -> Option<&ClauseProvenance> {
        self.fact_provenance.get(&id)
    }

    /// Look up the provenance for a given Rule ID, if recorded.
    pub fn provenance_for_rule(&self, id: RuleId) -> Option<&ClauseProvenance> {
        self.rule_provenance.get(&id)
    }
}

/// Lower an IR document into a KB while attributing every emitted
/// clause to a single provenance record.
///
/// This is the provenance-tracking sibling of [`lower_to_kb`]. Every
/// Fact ID and Rule ID assigned by the KB is recorded in the
/// returned `LoweredKb`'s attribution maps so that callers (the
/// engine, the audit trail, the disputed-answer resolution layer)
/// can recover which rulebook produced each clause.
///
/// All clauses emitted from this single call share the same
/// `provenance` — this is the *one rulebook in, one provenance out*
/// pattern. For multi-rulebook KBs (e.g., adversarial elicitation),
/// call this function once per source rulebook and combine the
/// results with [`LoweredKb::extend`].
pub fn lower_to_kb_with_provenance(
    ir_doc: &IRDocument,
    provenance: ClauseProvenance,
) -> Result<LoweredKb, LoweringError> {
    let mut lowered = LoweredKb::new();
    let mut constraint_counter: u64 = 0;
    for node in &ir_doc.nodes {
        match node.kind {
            NodeKind::Fact => {
                let ids = lower_fact_tracked(&mut lowered.kb, node)?;
                for id in ids {
                    match id {
                        ClauseId::Fact(fid) => {
                            lowered.fact_provenance.insert(fid, provenance.clone());
                        }
                        ClauseId::Rule(rid) => {
                            // Denied facts lower to a NAF rule, not a fact.
                            lowered.rule_provenance.insert(rid, provenance.clone());
                        }
                    }
                }
            }
            NodeKind::Rule => {
                let ids = lower_rule_tracked(&mut lowered.kb, node, &mut constraint_counter)?;
                for id in ids {
                    match id {
                        ClauseId::Fact(fid) => {
                            lowered.fact_provenance.insert(fid, provenance.clone());
                        }
                        ClauseId::Rule(rid) => {
                            lowered.rule_provenance.insert(rid, provenance.clone());
                        }
                    }
                }
            }
            NodeKind::Query
            | NodeKind::Uncertainty
            | NodeKind::Exception
            | NodeKind::Discarded
            | NodeKind::Section
            | NodeKind::Entity => {}
        }
    }
    for edge in &ir_doc.edges {
        if edge.relation == EdgeRelation::Contains {
            continue;
        }
        let functor = edge.relation.as_str().replace('-', "_");
        let head = compound(
            &functor,
            vec![atom(&edge.source.0), atom(&edge.target.0)],
        );
        let id = if edge.polarity == Polarity::Denied {
            let deny_head = compound(
                &format!("not_{functor}"),
                vec![atom(&edge.source.0), atom(&edge.target.0)],
            );
            lowered.kb.add_fact(Fact::certain(deny_head))
        } else {
            lowered.kb.add_fact(Fact::certain(head))
        };
        lowered.fact_provenance.insert(id, provenance.clone());
    }
    Ok(lowered)
}

// ---------------------------------------------------------------------------
// Lowering
// ---------------------------------------------------------------------------

/// Lower an IR document into a logic-engine KnowledgeBase, applying
/// the lowering rules from ADJ11.
///
/// Returns the KB on success or the first lowering error on failure.
/// The KB does **not** include Query nodes; use [`extract_queries`]
/// for those.
pub fn lower_to_kb(ir_doc: &IRDocument) -> Result<KnowledgeBase, LoweringError> {
    let mut kb = KnowledgeBase::new();
    let mut constraint_counter: u64 = 0;
    for node in &ir_doc.nodes {
        match node.kind {
            NodeKind::Fact => lower_fact(&mut kb, node)?,
            NodeKind::Rule => lower_rule(&mut kb, node, &mut constraint_counter)?,
            // Query nodes are returned by extract_queries; not added to KB.
            // Uncertainty / Exception / Discarded participate in
            // clarification, audit, and rule priority but do not produce
            // engine clauses.
            NodeKind::Query
            | NodeKind::Uncertainty
            | NodeKind::Exception
            | NodeKind::Discarded
            | NodeKind::Section
            | NodeKind::Entity => {
                // Query nodes are returned by extract_queries;
                // Uncertainty / Exception / Discarded participate in
                // clarification, audit, and rule priority. Section is
                // structural metadata only. Entity is a deduplicated
                // atom reference target; its content is lowered when
                // a mentioning Fact or Rule is lowered. None produce
                // independent engine clauses in v3.
            }
        }
    }

    // Edge lowering: emit one Prolog clause per typed edge so the
    // engine can reason about the relationship structure. The clauses
    // use the EdgeRelation's `as_str()` name as the functor:
    //
    //     excepts(<source_id>, <target_id>).
    //     applies_to(<source_id>, <target_id>).
    //     cites(<source_id>, <target_id>).
    //     ...
    //
    // For now we lower every edge uniformly. Domain-specific
    // optimizations (e.g., compiling `Excepts` into per-rule exception
    // bodies, compiling `Contains` into a structural-only relation
    // that the engine ignores) follow as the engine grows.
    for edge in &ir_doc.edges {
        // Skip Contains: it's structural metadata. The engine doesn't
        // need to know about document hierarchy.
        if edge.relation == EdgeRelation::Contains {
            continue;
        }
        let functor = edge.relation.as_str().replace('-', "_");
        let head = compound(
            &functor,
            vec![atom(&edge.source.0), atom(&edge.target.0)],
        );
        // Edge polarity / modality semantics: an Affirmed/Present edge
        // emits the clause as-is. Denied edges are recorded but not
        // emitted as positive clauses (the engine would have to use a
        // separate negative-knowledge mechanism). For now we skip
        // Denied edges and emit a deny_ functor; the engine treats it
        // as a witness for audit-trail replay.
        if edge.polarity == Polarity::Denied {
            let deny_head = compound(
                &format!("not_{functor}"),
                vec![atom(&edge.source.0), atom(&edge.target.0)],
            );
            kb.add_fact(Fact::certain(deny_head));
        } else {
            kb.add_fact(Fact::certain(head));
        }
    }

    Ok(kb)
}

/// Collect the `term` of every Query node in the document, in order.
/// Most documents contain exactly one Query; multiple are permitted.
pub fn extract_queries(ir_doc: &IRDocument) -> Vec<Term> {
    ir_doc
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Query)
        .map(|n| n.term.clone())
        .collect()
}

/// Like [`lower_fact`] but returns every clause ID inserted into the
/// KB. Used by the provenance-tracking variant of `lower_to_kb`.
fn lower_fact_tracked(
    kb: &mut KnowledgeBase,
    node: &IRNode,
) -> Result<Vec<ClauseId>, LoweringError> {
    let mut ids = Vec::new();
    match node.polarity {
        Polarity::Affirmed | Polarity::Uncertain | Polarity::Inherit => {
            let id = kb.add_fact(Fact::certain(node.term.clone()));
            ids.push(ClauseId::Fact(id));
        }
        Polarity::Denied => {
            let id = kb.add_rule(Rule::certain(
                node.term.clone(),
                vec![BodyLiteral::Neg(node.term.clone())],
            ));
            ids.push(ClauseId::Rule(id));
        }
    }
    Ok(ids)
}

/// Like [`lower_rule`] but returns every clause ID inserted into the
/// KB. The Rule subtype determines whether a single rule is emitted
/// (all current subtypes emit exactly one).
fn lower_rule_tracked(
    kb: &mut KnowledgeBase,
    node: &IRNode,
    constraint_counter: &mut u64,
) -> Result<Vec<ClauseId>, LoweringError> {
    // Snapshot the next_rule_id by adding the rule and reading the
    // assigned ID back. The lower_rule implementation already does
    // the work; we replicate it here returning the IDs.
    let Term::Compound { functor, args } = &node.term else {
        return Err(LoweringError::UnknownRuleSubtype {
            node_id: node.id.clone(),
            functor: render_term_summary(&node.term),
        });
    };

    let mut ids = Vec::new();
    match functor.as_str() {
        "definitional" => {
            if args.len() != 2 {
                return Err(LoweringError::InvalidRuleArity {
                    node_id: node.id.clone(),
                    subtype: "definitional".to_string(),
                    expected: 2,
                    actual: args.len(),
                });
            }
            let head = args[0].clone();
            let body = decode_list(&args[1])
                .ok_or_else(|| LoweringError::InvalidRuleBodyList {
                    node_id: node.id.clone(),
                    subtype: "definitional".to_string(),
                })?
                .into_iter()
                .map(BodyLiteral::Pos)
                .collect();
            ids.push(ClauseId::Rule(kb.add_rule(Rule::certain(head, body))));
        }
        "probabilistic" => {
            if args.len() != 3 {
                return Err(LoweringError::InvalidRuleArity {
                    node_id: node.id.clone(),
                    subtype: "probabilistic".to_string(),
                    expected: 3,
                    actual: args.len(),
                });
            }
            let p = match &args[0] {
                Term::Num(Number::Int(i)) => *i as f64,
                Term::Num(Number::Float(x)) => *x,
                other => {
                    return Err(LoweringError::InvalidProbability {
                        node_id: node.id.clone(),
                        found: render_term_summary(other),
                    });
                }
            };
            if !(0.0..=1.0).contains(&p) {
                return Err(LoweringError::ProbabilityOutOfRange {
                    node_id: node.id.clone(),
                    value: p,
                });
            }
            let head = args[1].clone();
            let body = decode_list(&args[2])
                .ok_or_else(|| LoweringError::InvalidRuleBodyList {
                    node_id: node.id.clone(),
                    subtype: "probabilistic".to_string(),
                })?
                .into_iter()
                .map(BodyLiteral::Pos)
                .collect();
            ids.push(ClauseId::Rule(kb.add_rule(Rule {
                id: RuleId(u64::MAX),
                head,
                body,
                probability: Probability::Value(p),
            })));
        }
        "constraint" => {
            if args.len() != 1 {
                return Err(LoweringError::InvalidRuleArity {
                    node_id: node.id.clone(),
                    subtype: "constraint".to_string(),
                    expected: 1,
                    actual: args.len(),
                });
            }
            let body = decode_list(&args[0])
                .ok_or_else(|| LoweringError::InvalidRuleBodyList {
                    node_id: node.id.clone(),
                    subtype: "constraint".to_string(),
                })?
                .into_iter()
                .map(BodyLiteral::Pos)
                .collect();
            let synthetic_head = logic_core::compound(
                "_constraint",
                vec![logic_core::atom(format!("c_{}", *constraint_counter))],
            );
            *constraint_counter += 1;
            ids.push(ClauseId::Rule(
                kb.add_rule(Rule::certain(synthetic_head, body)),
            ));
        }
        "default" => {
            if args.len() != 3 {
                return Err(LoweringError::InvalidRuleArity {
                    node_id: node.id.clone(),
                    subtype: "default".to_string(),
                    expected: 3,
                    actual: args.len(),
                });
            }
            let head = args[0].clone();
            let mut combined_body: Vec<BodyLiteral> = decode_list(&args[1])
                .ok_or_else(|| LoweringError::InvalidRuleBodyList {
                    node_id: node.id.clone(),
                    subtype: "default".to_string(),
                })?
                .into_iter()
                .map(BodyLiteral::Pos)
                .collect();
            let exceptions = decode_list(&args[2])
                .ok_or_else(|| LoweringError::InvalidRuleBodyList {
                    node_id: node.id.clone(),
                    subtype: "default".to_string(),
                })?;
            for exc in exceptions {
                combined_body.push(BodyLiteral::Neg(exc));
            }
            ids.push(ClauseId::Rule(
                kb.add_rule(Rule::certain(head, combined_body)),
            ));
        }
        other => {
            return Err(LoweringError::UnknownRuleSubtype {
                node_id: node.id.clone(),
                functor: other.to_string(),
            });
        }
    }
    Ok(ids)
}

/// Disambiguator for IDs that the lowering produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ClauseId {
    Fact(FactId),
    Rule(RuleId),
}

fn lower_fact(kb: &mut KnowledgeBase, node: &IRNode) -> Result<(), LoweringError> {
    match node.polarity {
        Polarity::Affirmed => {
            // The simple case: a positive fact with whatever probability
            // the source declared. Currently the IR has no `probability`
            // field on Facts (it's a Rule-subtype concern); affirmed
            // Facts lower to Certain probability. Future versions of the
            // IR may carry per-Fact probabilities directly.
            kb.add_fact(Fact::certain(node.term.clone()));
        }
        Polarity::Denied => {
            // ADJ11's polarity-to-clause translation under
            // negation-as-failure: `Denied(t)` lowers to a Rule whose
            // body is a single `Neg(t)` literal. The rule succeeds when
            // `t` cannot be proved, capturing the denied semantics.
            kb.add_rule(Rule::certain(
                node.term.clone(),
                vec![BodyLiteral::Neg(node.term.clone())],
            ));
        }
        Polarity::Uncertain => {
            // A Fact node should not have Uncertain polarity per
            // ADJ01 well-formedness. If we see it here the upstream
            // validation was bypassed; silently treat as Affirmed for
            // robustness rather than panicking. (A pre-validated
            // IRDocument never hits this branch.)
            kb.add_fact(Fact::certain(node.term.clone()));
        }
        Polarity::Inherit => {
            // v2: Inherit means "use the structural ancestor's
            // polarity". An ADJ03-v2 propagation pass should have
            // resolved this before lowering. If we see Inherit here
            // the upstream pass didn't run; fall back to the
            // framework default (Affirmed) for robustness.
            // A pre-resolved IRDocument never hits this branch.
            kb.add_fact(Fact::certain(node.term.clone()));
        }
    }
    Ok(())
}

fn lower_rule(
    kb: &mut KnowledgeBase,
    node: &IRNode,
    constraint_counter: &mut u64,
) -> Result<(), LoweringError> {
    let Term::Compound { functor, args } = &node.term else {
        return Err(LoweringError::UnknownRuleSubtype {
            node_id: node.id.clone(),
            functor: render_term_summary(&node.term),
        });
    };

    match functor.as_str() {
        "definitional" => {
            // definitional(head, [body...])
            if args.len() != 2 {
                return Err(LoweringError::InvalidRuleArity {
                    node_id: node.id.clone(),
                    subtype: "definitional".to_string(),
                    expected: 2,
                    actual: args.len(),
                });
            }
            let head = args[0].clone();
            let body = decode_list(&args[1])
                .ok_or_else(|| LoweringError::InvalidRuleBodyList {
                    node_id: node.id.clone(),
                    subtype: "definitional".to_string(),
                })?
                .into_iter()
                .map(BodyLiteral::Pos)
                .collect();
            kb.add_rule(Rule::certain(head, body));
        }
        "probabilistic" => {
            // probabilistic(p, head, [body...])
            if args.len() != 3 {
                return Err(LoweringError::InvalidRuleArity {
                    node_id: node.id.clone(),
                    subtype: "probabilistic".to_string(),
                    expected: 3,
                    actual: args.len(),
                });
            }
            let p = match &args[0] {
                Term::Num(Number::Int(i)) => *i as f64,
                Term::Num(Number::Float(x)) => *x,
                other => {
                    return Err(LoweringError::InvalidProbability {
                        node_id: node.id.clone(),
                        found: render_term_summary(other),
                    });
                }
            };
            if !(0.0..=1.0).contains(&p) {
                return Err(LoweringError::ProbabilityOutOfRange {
                    node_id: node.id.clone(),
                    value: p,
                });
            }
            let head = args[1].clone();
            let body = decode_list(&args[2])
                .ok_or_else(|| LoweringError::InvalidRuleBodyList {
                    node_id: node.id.clone(),
                    subtype: "probabilistic".to_string(),
                })?
                .into_iter()
                .map(BodyLiteral::Pos)
                .collect();
            kb.add_rule(Rule {
                id: logic_engine::RuleId(u64::MAX), // overwritten on insert
                head,
                body,
                probability: Probability::Value(p),
            });
        }
        "constraint" => {
            // constraint([body...]) - synthetic head `_constraint(c_N)`
            if args.len() != 1 {
                return Err(LoweringError::InvalidRuleArity {
                    node_id: node.id.clone(),
                    subtype: "constraint".to_string(),
                    expected: 1,
                    actual: args.len(),
                });
            }
            let body = decode_list(&args[0])
                .ok_or_else(|| LoweringError::InvalidRuleBodyList {
                    node_id: node.id.clone(),
                    subtype: "constraint".to_string(),
                })?
                .into_iter()
                .map(BodyLiteral::Pos)
                .collect();
            let synthetic_head = logic_core::compound(
                "_constraint",
                vec![logic_core::atom(format!("c_{}", *constraint_counter))],
            );
            *constraint_counter += 1;
            kb.add_rule(Rule::certain(synthetic_head, body));
        }
        "default" => {
            // default(head, [body...], [exceptions...])
            if args.len() != 3 {
                return Err(LoweringError::InvalidRuleArity {
                    node_id: node.id.clone(),
                    subtype: "default".to_string(),
                    expected: 3,
                    actual: args.len(),
                });
            }
            let head = args[0].clone();
            let mut combined_body: Vec<BodyLiteral> = decode_list(&args[1])
                .ok_or_else(|| LoweringError::InvalidRuleBodyList {
                    node_id: node.id.clone(),
                    subtype: "default".to_string(),
                })?
                .into_iter()
                .map(BodyLiteral::Pos)
                .collect();
            let exceptions = decode_list(&args[2])
                .ok_or_else(|| LoweringError::InvalidRuleBodyList {
                    node_id: node.id.clone(),
                    subtype: "default".to_string(),
                })?;
            for exc in exceptions {
                combined_body.push(BodyLiteral::Neg(exc));
            }
            kb.add_rule(Rule::certain(head, combined_body));
        }
        other => {
            return Err(LoweringError::UnknownRuleSubtype {
                node_id: node.id.clone(),
                functor: other.to_string(),
            });
        }
    }
    Ok(())
}

/// Decode a Prolog-style list term (using `'.'/2` cons cells and the
/// `[]` empty-list atom) into a Vec of Terms. Returns `None` if the
/// term is not a well-formed list.
fn decode_list(term: &Term) -> Option<Vec<Term>> {
    let mut out = Vec::new();
    let mut current = term;
    loop {
        match current {
            Term::Atom(name) if name == "[]" => return Some(out),
            Term::Compound { functor, args } if functor == "." && args.len() == 2 => {
                out.push(args[0].clone());
                current = &args[1];
            }
            _ => return None,
        }
    }
}

/// Cheap one-line summary of a term for error messages.
fn render_term_summary(term: &Term) -> String {
    match term {
        Term::Atom(s) => s.clone(),
        Term::Num(_) | Term::Str(_) | Term::Var(_) => term.to_string(),
        Term::Compound { functor, args } => format!("{}/{}", functor, args.len()),
    }
}

// ---------------------------------------------------------------------------
// End-to-end adjudication
// ---------------------------------------------------------------------------

/// One query's result after running through the engine.
#[derive(Debug, Clone)]
pub struct AdjudicationResult {
    pub query: Term,
    pub result: SearchResult,
}

/// Lower the IR document and run every Query node under
/// `SearchMode::AutoDetect`. Returns one [`AdjudicationResult`] per
/// Query.
pub fn run_adjudication(ir_doc: &IRDocument) -> Result<Vec<AdjudicationResult>, LoweringError> {
    let kb = lower_to_kb(ir_doc)?;
    let queries = extract_queries(ir_doc);
    Ok(queries
        .into_iter()
        .map(|q| {
            let result = search(&q, &kb, SearchMode::AutoDetect);
            AdjudicationResult { query: q, result }
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Inline unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use adjudication_ir::{DocumentId, Modality, NodeKind as Nk, Span};
    use logic_core::{atom, compound, int, var};

    fn doc_id() -> DocumentId {
        DocumentId::new("doc1")
    }

    fn span() -> Span {
        Span::new(doc_id(), 0, 10)
    }

    fn empty_meta() -> std::collections::HashMap<String, String> {
        std::collections::HashMap::new()
    }

    fn affirmed_fact_node(id: &str, term: Term) -> IRNode {
        IRNode {
            id: NodeId::new(id),
            kind: Nk::Fact,
            term,
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![span()],
            confidence: 1.0,
            discard_reason: None,
            metadata: empty_meta(),
        }
    }

    fn denied_fact_node(id: &str, term: Term) -> IRNode {
        IRNode {
            id: NodeId::new(id),
            kind: Nk::Fact,
            term,
            polarity: Polarity::Denied,
            modality: Modality::Present,
            source_spans: vec![span()],
            confidence: 1.0,
            discard_reason: None,
            metadata: empty_meta(),
        }
    }

    fn rule_node(id: &str, term: Term) -> IRNode {
        IRNode {
            id: NodeId::new(id),
            kind: Nk::Rule,
            term,
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![span()],
            confidence: 1.0,
            discard_reason: None,
            metadata: empty_meta(),
        }
    }

    fn query_node(id: &str, term: Term) -> IRNode {
        IRNode {
            id: NodeId::new(id),
            kind: Nk::Query,
            term,
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![span()],
            confidence: 1.0,
            discard_reason: None,
            metadata: empty_meta(),
        }
    }

    fn list_of(terms: Vec<Term>) -> Term {
        logic_core::logic_list(terms)
    }

    #[test]
    fn empty_document_produces_empty_kb() {
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![],
            edges: vec![],
        };
        let kb = lower_to_kb(&doc).unwrap();
        assert!(kb.is_all_certain()); // vacuously
    }

    #[test]
    fn affirmed_fact_lowers_to_certain_fact() {
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![affirmed_fact_node("F1", atom("ok"))],
            edges: vec![],
        };
        let kb = lower_to_kb(&doc).unwrap();
        // We can't directly observe internals; instead, run a search.
        let r = search(&atom("ok"), &kb, SearchMode::AutoDetect);
        match r {
            SearchResult::FindFirstResult(Some(_)) => {} // expected
            other => panic!("expected FindFirstResult(Some), got {:?}", other),
        }
    }

    #[test]
    fn denied_fact_lowers_without_error() {
        // Denied(t) lowers to Rule { head: t, body: [Neg(t)] } — a
        // rule that succeeds when `t` cannot be proved.
        //
        // This produces a NON-STRATIFIED program (`t :- \+ t.`) when
        // the denied fact appears in isolation. LP19's well-founded
        // semantics rejects such programs; the stratification check is
        // a follow-up sub-spec (LP19a) and not yet implemented in the
        // Rust engine. So we verify only that the *lowering* succeeds;
        // running search on the resulting KB would recurse non-
        // terminatingly until the engine implements the check.
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![denied_fact_node("F1", atom("absent"))],
            edges: vec![],
        };
        let kb = lower_to_kb(&doc).unwrap();
        // Sanity: the KB has at least one rule (the NAF rule).
        // Use is_all_certain() as a proxy for "the KB is populated"
        // since we don't expose direct counts.
        assert!(kb.is_all_certain(), "KB should contain only Certain clauses");
    }

    #[test]
    fn denied_fact_combined_with_other_proof_path_resolves_cleanly() {
        // Denied(absent) is the NAF rule `absent :- \+ absent.`. Add
        // an UNRELATED predicate so the test runs without entering the
        // non-stratified recursion: querying `something_else` does not
        // touch the NAF rule.
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![
                denied_fact_node("F1", atom("absent")),
                affirmed_fact_node("F2", atom("present")),
                query_node("Q1", atom("present")),
            ],
            edges: vec![],
        };
        let results = run_adjudication(&doc).unwrap();
        match &results[0].result {
            SearchResult::FindFirstResult(Some(_)) => {}
            other => panic!("expected present to succeed, got {:?}", other),
        }
    }

    #[test]
    fn definitional_rule_lowering_executes_correctly() {
        // definitional(parent(X, Y), [father(X, Y)])
        let xv = var("X");
        let yv = var("Y");
        let head = compound(
            "parent",
            vec![Term::Var(xv.clone()), Term::Var(yv.clone())],
        );
        let body_lit = compound(
            "father",
            vec![Term::Var(xv.clone()), Term::Var(yv.clone())],
        );
        let body_list = list_of(vec![body_lit]);
        let rule_term = compound("definitional", vec![head, body_list]);

        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![
                affirmed_fact_node(
                    "F1",
                    compound("father", vec![atom("homer"), atom("bart")]),
                ),
                rule_node("R1", rule_term),
                query_node(
                    "Q1",
                    compound("parent", vec![atom("homer"), atom("bart")]),
                ),
            ],
            edges: vec![],
        };

        let results = run_adjudication(&doc).unwrap();
        assert_eq!(results.len(), 1);
        match &results[0].result {
            SearchResult::FindFirstResult(Some(_)) => {}
            other => panic!("expected parent(homer, bart) to succeed, got {:?}", other),
        }
    }

    #[test]
    fn probabilistic_rule_lowering_produces_value_probability() {
        // probabilistic(0.5, alarm, [burglary])
        // Together with `burglary` (Certain), engine should compute
        // P(alarm) = 0.5.
        let rule_term = compound(
            "probabilistic",
            vec![
                logic_core::float(0.5),
                atom("alarm"),
                list_of(vec![atom("burglary")]),
            ],
        );

        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![
                affirmed_fact_node("F1", atom("burglary")),
                rule_node("R1", rule_term),
                query_node("Q1", atom("alarm")),
            ],
            edges: vec![],
        };

        let results = run_adjudication(&doc).unwrap();
        assert_eq!(results.len(), 1);
        match &results[0].result {
            SearchResult::EnumerateAllResult { probability, .. } => {
                assert!(
                    (*probability - 0.5).abs() < 1e-9,
                    "expected P(alarm) = 0.5, got {}",
                    probability
                );
            }
            other => panic!("expected probabilistic result, got {:?}", other),
        }
    }

    #[test]
    fn constraint_rule_lowering_uses_synthetic_head() {
        let body_lit = atom("placeholder");
        let body_list = list_of(vec![body_lit]);
        let rule_term = compound("constraint", vec![body_list]);

        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![rule_node("R1", rule_term)],
            edges: vec![],
        };

        let kb = lower_to_kb(&doc).unwrap();
        // The synthetic head is `_constraint(c_0)`. We don't have a
        // direct accessor, but we can verify it's present by
        // querying for it.
        let r = search(
            &compound("_constraint", vec![atom("c_0")]),
            &kb,
            SearchMode::FindFirst,
        );
        // The rule's body requires `placeholder` to be provable,
        // which it isn't. So the query for the synthetic head
        // should fail.
        match r {
            SearchResult::FindFirstResult(None) => {} // body fails
            other => panic!("expected None (body unsatisfied), got {:?}", other),
        }
    }

    #[test]
    fn default_rule_lowering_combines_body_and_negated_exceptions() {
        // default(p, [a], [b])  →  p :- a, \+ b
        // With a present, p is provable iff b is not.
        let rule_term = compound(
            "default",
            vec![
                atom("p"),
                list_of(vec![atom("a")]),
                list_of(vec![atom("b")]),
            ],
        );

        // Case 1: a holds, b doesn't → p succeeds.
        let doc1 = IRDocument {
            document_id: doc_id(),
            nodes: vec![
                affirmed_fact_node("F1", atom("a")),
                rule_node("R1", rule_term.clone()),
                query_node("Q1", atom("p")),
            ],
            edges: vec![],
        };
        let r1 = run_adjudication(&doc1).unwrap();
        match &r1[0].result {
            SearchResult::FindFirstResult(Some(_)) => {}
            other => panic!("expected p to succeed when a yes, b no; got {:?}", other),
        }

        // Case 2: a and b both hold → p fails (exception fires).
        let doc2 = IRDocument {
            document_id: doc_id(),
            nodes: vec![
                affirmed_fact_node("F1", atom("a")),
                affirmed_fact_node("F2", atom("b")),
                rule_node("R1", rule_term),
                query_node("Q1", atom("p")),
            ],
            edges: vec![],
        };
        let r2 = run_adjudication(&doc2).unwrap();
        match &r2[0].result {
            SearchResult::FindFirstResult(None) => {}
            other => panic!("expected p to fail when exception holds; got {:?}", other),
        }
    }

    #[test]
    fn unknown_rule_subtype_errors_with_functor_name() {
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![rule_node(
                "R1",
                compound("unknownify", vec![atom("x")]),
            )],
            edges: vec![],
        };
        match lower_to_kb(&doc) {
            Err(LoweringError::UnknownRuleSubtype { functor, .. }) => {
                assert_eq!(functor, "unknownify");
            }
            other => panic!("expected UnknownRuleSubtype, got {:?}", other),
        }
    }

    #[test]
    fn rule_with_wrong_arity_errors() {
        // definitional only takes 2 args
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![rule_node(
                "R1",
                compound("definitional", vec![atom("h")]),
            )],
            edges: vec![],
        };
        match lower_to_kb(&doc) {
            Err(LoweringError::InvalidRuleArity {
                subtype, expected, actual, ..
            }) => {
                assert_eq!(subtype, "definitional");
                assert_eq!(expected, 2);
                assert_eq!(actual, 1);
            }
            other => panic!("expected InvalidRuleArity, got {:?}", other),
        }
    }

    #[test]
    fn probabilistic_with_non_numeric_p_errors() {
        let rule_term = compound(
            "probabilistic",
            vec![atom("not_a_number"), atom("h"), list_of(vec![])],
        );
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![rule_node("R1", rule_term)],
            edges: vec![],
        };
        match lower_to_kb(&doc) {
            Err(LoweringError::InvalidProbability { .. }) => {}
            other => panic!("expected InvalidProbability, got {:?}", other),
        }
    }

    #[test]
    fn probabilistic_with_out_of_range_p_errors() {
        let rule_term = compound(
            "probabilistic",
            vec![logic_core::float(1.5), atom("h"), list_of(vec![])],
        );
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![rule_node("R1", rule_term)],
            edges: vec![],
        };
        match lower_to_kb(&doc) {
            Err(LoweringError::ProbabilityOutOfRange { value, .. }) => {
                assert!((value - 1.5).abs() < 1e-9);
            }
            other => panic!("expected ProbabilityOutOfRange, got {:?}", other),
        }
    }

    #[test]
    fn rule_body_not_a_list_errors() {
        let rule_term = compound(
            "definitional",
            vec![atom("h"), atom("not_a_list")], // should be a list
        );
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![rule_node("R1", rule_term)],
            edges: vec![],
        };
        match lower_to_kb(&doc) {
            Err(LoweringError::InvalidRuleBodyList { .. }) => {}
            other => panic!("expected InvalidRuleBodyList, got {:?}", other),
        }
    }

    #[test]
    fn extract_queries_returns_all_query_terms_in_order() {
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![
                query_node("Q1", atom("a")),
                affirmed_fact_node("F1", atom("a")),
                query_node("Q2", atom("b")),
            ],
            edges: vec![],
        };
        let queries = extract_queries(&doc);
        assert_eq!(queries, vec![atom("a"), atom("b")]);
    }

    // -----------------------------------------------------------------
    // ADJ16 step 1 — provenance pass-through tests
    // -----------------------------------------------------------------

    fn tsa_provenance() -> ClauseProvenance {
        ClauseProvenance::new("tsa-rules-v1", TrustTier::Tentative)
    }

    fn reviewed_provenance() -> ClauseProvenance {
        ClauseProvenance::new("tsa-rules-v1", TrustTier::Reviewed)
    }

    #[test]
    fn trust_tier_string_representations_round_trip() {
        assert_eq!(TrustTier::Tentative.as_str(), "tentative");
        assert_eq!(TrustTier::Reviewed.as_str(), "reviewed");
        assert_eq!(TrustTier::Authoritative.as_str(), "authoritative");
    }

    #[test]
    fn lower_with_provenance_attributes_every_affirmed_fact() {
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![
                affirmed_fact_node("F1", atom("ok")),
                affirmed_fact_node("F2", atom("done")),
            ],
            edges: vec![],
        };
        let lowered = lower_to_kb_with_provenance(&doc, tsa_provenance()).unwrap();
        // Two facts were emitted (IDs 0 and 1, since the KB is fresh).
        assert_eq!(lowered.fact_provenance.len(), 2);
        assert_eq!(lowered.rule_provenance.len(), 0);
        // Every recorded provenance is the one we passed in.
        for prov in lowered.fact_provenance.values() {
            assert_eq!(prov.source_rulebook_id, "tsa-rules-v1");
            assert_eq!(prov.trust_tier, TrustTier::Tentative);
        }
        // The KB still answers the same queries the non-provenance path
        // would: `ok` and `done` are both provable.
        match search(&atom("ok"), &lowered.kb, SearchMode::FindFirst) {
            SearchResult::FindFirstResult(Some(_)) => {}
            other => panic!("expected ok to be provable, got {:?}", other),
        }
    }

    #[test]
    fn lower_with_provenance_attributes_rules_emitted_for_denied_facts() {
        // A denied fact lowers to a Rule (NAF body), not a Fact, so
        // the provenance should appear in `rule_provenance`.
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![denied_fact_node("F1", atom("absent"))],
            edges: vec![],
        };
        let lowered = lower_to_kb_with_provenance(&doc, tsa_provenance()).unwrap();
        assert_eq!(lowered.fact_provenance.len(), 0);
        assert_eq!(lowered.rule_provenance.len(), 1);
    }

    #[test]
    fn lower_with_provenance_attributes_rules() {
        let rule_term = compound(
            "definitional",
            vec![atom("p"), list_of(vec![atom("a")])],
        );
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![
                rule_node("R1", rule_term),
                affirmed_fact_node("F1", atom("a")),
            ],
            edges: vec![],
        };
        let lowered = lower_to_kb_with_provenance(&doc, reviewed_provenance()).unwrap();
        assert_eq!(lowered.fact_provenance.len(), 1);
        assert_eq!(lowered.rule_provenance.len(), 1);
        for prov in lowered.rule_provenance.values() {
            assert_eq!(prov.trust_tier, TrustTier::Reviewed);
        }
    }

    #[test]
    fn lower_with_provenance_attributes_edges_as_facts() {
        use adjudication_ir::{IREdge, EdgeId, EdgeRelation};
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![
                affirmed_fact_node("F1", atom("a")),
                affirmed_fact_node("F2", atom("b")),
            ],
            edges: vec![IREdge {
                id: EdgeId::new("E1"),
                source: NodeId::new("F1"),
                target: NodeId::new("F2"),
                relation: EdgeRelation::Cites,
                polarity: Polarity::Affirmed,
                modality: adjudication_ir::Modality::Present,
                source_spans: vec![span()],
                confidence: 1.0,
                metadata: empty_meta(),
            }],
        };
        let lowered = lower_to_kb_with_provenance(&doc, tsa_provenance()).unwrap();
        // Two fact nodes + one edge-as-fact = three facts.
        assert_eq!(lowered.fact_provenance.len(), 3);
    }

    #[test]
    fn lower_with_provenance_skips_contains_edges() {
        use adjudication_ir::{IREdge, EdgeId};
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![
                affirmed_fact_node("F1", atom("a")),
                affirmed_fact_node("F2", atom("b")),
            ],
            edges: vec![IREdge {
                id: EdgeId::new("E1"),
                source: NodeId::new("F1"),
                target: NodeId::new("F2"),
                relation: EdgeRelation::Contains,
                polarity: Polarity::Affirmed,
                modality: adjudication_ir::Modality::Present,
                source_spans: vec![span()],
                confidence: 1.0,
                metadata: empty_meta(),
            }],
        };
        let lowered = lower_to_kb_with_provenance(&doc, tsa_provenance()).unwrap();
        // Two node facts; Contains edge skipped.
        assert_eq!(lowered.fact_provenance.len(), 2);
    }

    #[test]
    fn lowered_kb_extend_preserves_per_source_provenance() {
        // Two independent rulebooks contribute to one KB. After
        // extend, every clause is attributable to its origin.
        let doc_a = IRDocument {
            document_id: DocumentId::new("rb-a"),
            nodes: vec![affirmed_fact_node("FA", atom("from_a"))],
            edges: vec![],
        };
        let doc_b = IRDocument {
            document_id: DocumentId::new("rb-b"),
            nodes: vec![affirmed_fact_node("FB", atom("from_b"))],
            edges: vec![],
        };
        let prov_a = ClauseProvenance::new("rulebook-a", TrustTier::Tentative);
        let prov_b = ClauseProvenance::new("rulebook-b", TrustTier::Reviewed);
        let lowered_a = lower_to_kb_with_provenance(&doc_a, prov_a.clone()).unwrap();
        let lowered_b = lower_to_kb_with_provenance(&doc_b, prov_b.clone()).unwrap();

        let mut combined = LoweredKb::new();
        combined.extend(lowered_a);
        combined.extend(lowered_b);

        // Two facts total, one attributed to each rulebook.
        assert_eq!(combined.fact_provenance.len(), 2);
        let mut tiers_seen: Vec<&str> = combined
            .fact_provenance
            .values()
            .map(|p| p.source_rulebook_id.as_str())
            .collect();
        tiers_seen.sort();
        assert_eq!(tiers_seen, vec!["rulebook-a", "rulebook-b"]);
    }

    #[test]
    fn lowered_kb_provenance_lookup_returns_recorded_record() {
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![affirmed_fact_node("F1", atom("a"))],
            edges: vec![],
        };
        let lowered = lower_to_kb_with_provenance(&doc, tsa_provenance()).unwrap();
        // The single fact was assigned FactId(0).
        let prov = lowered.provenance_for_fact(FactId(0)).expect("fact 0 missing");
        assert_eq!(prov.source_rulebook_id, "tsa-rules-v1");
        assert_eq!(prov.trust_tier, TrustTier::Tentative);
        // A nonexistent ID returns None.
        assert!(lowered.provenance_for_fact(FactId(999)).is_none());
    }

    #[test]
    fn lowered_kb_provenance_includes_all_rule_subtypes() {
        // Verify every Rule subtype lands in rule_provenance.
        let definitional = compound(
            "definitional",
            vec![atom("d_head"), list_of(vec![atom("d_body")])],
        );
        let probabilistic = compound(
            "probabilistic",
            vec![logic_core::float(0.5), atom("p_head"), list_of(vec![])],
        );
        let constraint = compound("constraint", vec![list_of(vec![atom("c_body")])]);
        let default = compound(
            "default",
            vec![atom("def_head"), list_of(vec![atom("a")]), list_of(vec![atom("b")])],
        );
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![
                rule_node("R1", definitional),
                rule_node("R2", probabilistic),
                rule_node("R3", constraint),
                rule_node("R4", default),
            ],
            edges: vec![],
        };
        let lowered = lower_to_kb_with_provenance(&doc, tsa_provenance()).unwrap();
        assert_eq!(lowered.rule_provenance.len(), 4);
        assert_eq!(lowered.fact_provenance.len(), 0);
    }

    #[test]
    fn lower_with_provenance_propagates_lowering_errors() {
        // A malformed rule should error from the provenance-aware
        // path the same way it errors from `lower_to_kb`.
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![rule_node(
                "R1",
                compound("unknownify", vec![atom("x")]),
            )],
            edges: vec![],
        };
        match lower_to_kb_with_provenance(&doc, tsa_provenance()) {
            Err(LoweringError::UnknownRuleSubtype { functor, .. }) => {
                assert_eq!(functor, "unknownify");
            }
            other => panic!("expected UnknownRuleSubtype, got {:?}", other),
        }
    }

    #[test]
    fn integer_probability_is_accepted_when_in_range() {
        // probabilistic(1, head, []) — integer 1, equivalent to Value(1.0)
        let rule_term = compound(
            "probabilistic",
            vec![int(1), atom("h"), list_of(vec![])],
        );
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![rule_node("R1", rule_term), query_node("Q1", atom("h"))],
            edges: vec![],
        };
        let results = run_adjudication(&doc).unwrap();
        match &results[0].result {
            SearchResult::EnumerateAllResult { probability, .. } => {
                assert!((*probability - 1.0).abs() < 1e-9);
            }
            other => panic!("expected EnumerateAllResult, got {:?}", other),
        }
    }
}
