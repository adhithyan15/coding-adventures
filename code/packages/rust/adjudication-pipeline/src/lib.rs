// LoweringError carries detail strings on every variant; the
// audit-trail discipline (every error logged with full context) is
// worth the cost as long as the error path isn't hot. Same trade-off
// as `llm-gateway` and the adjudication checker crates.
#![allow(clippy::result_large_err)]

//! # adjudication-pipeline — end-to-end orchestrator
//!
//! Composes the framework's checker passes, the engine connector, and
//! the audit-trail schema into a single function: given a normalized
//! document and an IR document, produce a typed [`Verdict`] and a
//! fully-populated [`adjudication_audit_trail::AuditTrail`].
//!
//! This is the **semantic source map** running end-to-end. Today's
//! v0.1.0 composes the slices that have already shipped:
//!
//!   * ADJ02 v2 coverage check
//!     ([`adjudication_coverage::check_coverage`]).
//!   * ADJ03 v2 polarity/modality propagation check
//!     ([`adjudication_polarity_modality::check_propagation`]).
//!   * The engine connector
//!     ([`adjudication_connector::run_adjudication`]).
//!   * ADJ07 audit-trail population
//!     ([`adjudication_audit_trail::AuditTrail`]).
//!
//! ADJ04 (round-trip) runs **when** the caller provides a
//! [`GatewayConfig`] with `Renderer` + `Nli` clients registered. If
//! no gateway is supplied (or those roles aren't bound), ADJ04 is
//! recorded as [`PassOutcome::Skipped`] with `pass_version =
//! "not-yet-wired"`, preserving the v0.1/v0.2 behaviour.
//!
//! ADJ05 (adversarial) still records as `Skipped` — it needs a
//! second, family-disjoint `Adversary` client and lands in v0.4.
//!
//! ## What this crate deliberately does NOT do (yet)
//!
//! - **Extraction.** Today's pipeline accepts a pre-built
//!   `IRDocument`. v0.2 will wire `llm_primitives::decompose_text`
//!   in front so the input is just `(String, DocumentId)`.
//! - **ADJ06 clarification dialogue.** A failing check produces a
//!   `Verdict::Blocked` with the violations attached — the caller
//!   handles the conversation loop.
//! - **Persistence.** The pipeline returns an in-memory `AuditTrail`;
//!   the deployment chooses how to write it (inline response,
//!   append-only log, content-addressed storage).

use adjudication_audit_trail::{
    AdjudicationId, AdjudicationOutcome, AuditTrail, CheckerResult, ClarificationKind, Document,
    DocumentId, EngineArtifacts, IrNode, KbSummary, NodeId, NormalizationRecord, PassName,
    PassOutcome, SearchMode as TrailSearchMode, SearchMode, Violation,
};
use adjudication_connector::{
    lower_to_kb_with_provenance, AdjudicationResult, ClauseProvenance, LoweredKb, TrustTier,
};
use adjudication_coverage::{check_coverage, CoverageResult, CoverageViolation, Document as CovDocument};
use adjudication_ir::{IRDocument, IRNode, NodeId as IRNodeId};
use adjudication_polarity_modality::{
    check_propagation, PropagationResult, PropagationViolation, PropagationWarning,
};
use adjudication_round_trip::{
    check_round_trip, CheckError as RoundTripCheckError, CheckOptions as RoundTripOptions,
    RoundTripResult, RoundTripViolation,
};
use adjudication_adversarial::{
    check_adversarial, AdversarialResult, AdversarialViolation, CheckError as AdversarialCheckError,
    CheckOptions as AdversarialOptions,
};
use llm_primitives::{GatewayConfig, Role as PrimitiveRole};

// `SearchMode` and `TrailSearchMode` collapse to the same trail-side
// enum; alias for clarity.
const _: () = {
    let _ = TrailSearchMode::AutoDetect;
};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// What you feed the pipeline.
#[derive(Debug, Clone)]
pub struct PipelineInput {
    /// The normalized source document. Spans in `ir_document` are
    /// byte offsets into `document.normalized_text`.
    pub document: PipelineDocument,
    /// The extracted hierarchical IR. v0.1 takes this pre-built; v0.2
    /// will replace it with a `source_text: String` once the
    /// `decompose_text` primitive lands.
    pub ir_document: IRDocument,
}

/// Stand-in for the `adjudication_coverage::Document` plus the
/// audit-trail metadata. Keeping the pipeline's input type minimal so
/// the deployment doesn't have to import the coverage crate just to
/// build a pipeline input.
#[derive(Debug, Clone)]
pub struct PipelineDocument {
    pub id: String,
    pub name: String,
    pub received_at: String,
    pub normalized_text: String,
    pub normalization_pipeline: String,
    pub normalization_version: String,
}

/// What the pipeline produces.
///
/// `clause_provenance` is `None` for the legacy entry points
/// ([`run`], [`run_with_gateway`]) and `Some` for
/// [`run_with_rulebooks`] (ADJ16 step 2). Each `FactId` / `RuleId`
/// that ended up in the engine's KB is keyed in the maps to the
/// source rulebook that produced it.
///
/// `disputed_answers` (ADJ16 step 3) lists queries whose proof DAGs
/// contain multiple proofs that (a) come from different rulebooks
/// and (b) produce different variable bindings. An empty vec means
/// no dispute was detected — either no rulebooks were attached, the
/// engine ran in `FindFirst` mode (only one proof returned), or all
/// proofs agreed.
#[derive(Debug)]
pub struct PipelineOutput {
    pub verdict: Verdict,
    pub audit_trail: AuditTrail,
    pub clause_provenance: Option<ClauseProvenanceTable>,
    pub disputed_answers: Vec<DisputedAnswer>,
}

// ---------------------------------------------------------------------------
// ADJ16 step 3 — DisputedAnswer
// ---------------------------------------------------------------------------

/// A query whose engine proofs disagree across rulebooks.
///
/// Surfaced when [`run_with_rulebooks`] returns multiple proofs (in
/// `SearchMode::EnumerateAll`) and the proofs' clause-provenance
/// attribution shows that distinct rulebooks led to distinct variable
/// bindings. This is the data shape [ADJ16 §"Open questions" §2]
/// names as `DisputedAnswer`: both proofs travel through the audit
/// trail; the caller (or a future ADJ06 dialogue) decides resolution.
#[derive(Debug, Clone)]
pub struct DisputedAnswer {
    /// The query that produced disagreeing proofs.
    pub query: logic_core::Term,
    /// One candidate per distinct (binding, source_rulebook_set)
    /// pairing. Identical proofs from the same rulebooks are
    /// de-duplicated in [`detect_disputes`].
    pub candidates: Vec<DisputeCandidate>,
    /// What kind of intervention is needed to resolve the dispute.
    pub resolution_required: ResolutionRequirement,
}

/// One candidate verdict inside a [`DisputedAnswer`].
#[derive(Debug, Clone)]
pub struct DisputeCandidate {
    /// Variable bindings under which this candidate's proof
    /// succeeded.
    pub bindings: logic_core::Substitution,
    /// Fact IDs cited by this candidate's proof (deduplicated).
    pub via_facts: Vec<logic_engine::FactId>,
    /// Rule IDs cited by this candidate's proof (deduplicated).
    pub via_rules: Vec<logic_engine::RuleId>,
    /// Source rulebooks that contributed to this candidate's proof,
    /// sorted lexicographically for stable display.
    pub source_rulebooks: Vec<String>,
}

/// What kind of intervention resolves a dispute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionRequirement {
    /// Multiple rulebooks produced conflicting bindings — needs
    /// expert review per ADJ09's review workflow. The default for
    /// v0.6.
    HumanReview,
    /// (Future, not yet emitted by `detect_disputes`.) A higher-trust
    /// rulebook in the candidate set wins automatically; the named
    /// rulebook is the winner. Trust-tier dominance is a
    /// deployment-policy decision, not a framework default.
    TrustTierDominates { winner_rulebook_id: String },
}

/// Walk every answer's proof DAG and surface disputes.
///
/// A dispute is recorded when, for a single query, the proof DAG
/// contains **at least two proofs** whose:
/// - source-rulebook attributions are not identical sets, AND
/// - variable bindings differ (proofs producing the same bindings
///   from different rulebooks are corroborating, not disputing).
///
/// Returns an empty vec when no answer has multiple proofs (e.g.,
/// the engine ran in `FindFirst` mode), when all proofs share the
/// same rulebook attribution, or when all proofs produce identical
/// bindings.
pub fn detect_disputes(
    answers: &[AdjudicationResult],
    provenance: &ClauseProvenanceTable,
) -> Vec<DisputedAnswer> {
    let mut out = Vec::new();
    for answer in answers {
        let dag = match &answer.result {
            logic_engine::SearchResult::EnumerateAllResult { dag, .. } => dag,
            _ => continue,
        };
        if dag.proofs.len() < 2 {
            continue;
        }

        let mut candidates: Vec<DisputeCandidate> = Vec::new();
        for proof in &dag.proofs {
            let mut rulebooks: std::collections::BTreeSet<String> =
                std::collections::BTreeSet::new();
            for f_id in &proof.via_facts {
                if let Some(prov) = provenance.fact_provenance.get(f_id) {
                    rulebooks.insert(prov.source_rulebook_id.clone());
                }
            }
            for r_id in &proof.via_rules {
                if let Some(prov) = provenance.rule_provenance.get(r_id) {
                    rulebooks.insert(prov.source_rulebook_id.clone());
                }
            }
            candidates.push(DisputeCandidate {
                bindings: proof.bindings.clone(),
                via_facts: proof.via_facts.clone(),
                via_rules: proof.via_rules.clone(),
                source_rulebooks: rulebooks.into_iter().collect(),
            });
        }

        // De-duplicate: if two candidates have identical bindings AND
        // identical rulebook sets, they are the same candidate and
        // shouldn't double-count. (Equal bindings from different
        // rulebooks = corroborating, kept separate; the dispute test
        // below uses the multiset.)
        let mut unique: Vec<DisputeCandidate> = Vec::new();
        for c in candidates {
            let already_present = unique.iter().any(|u| {
                u.bindings == c.bindings && u.source_rulebooks == c.source_rulebooks
            });
            if !already_present {
                unique.push(c);
            }
        }

        // Dispute test: there exists a *pair* of candidates whose
        // rulebook attributions differ AND whose bindings differ.
        // The joint per-pair check (vs evaluating the two conditions
        // globally) avoids a subtle false-positive where one
        // rulebook's within-rulebook ambiguity gets paired with an
        // unrelated second rulebook's corroborating proof. The
        // standard semantic for "two rulebooks disagree" is: there
        // is a pair (p_i, p_j) such that p_i and p_j cite different
        // rulebook sets AND produce different bindings for the
        // query variables.
        //
        // Same bindings from different rulebooks = corroborating
        // (not a dispute). Different bindings from the same rulebook
        // = within-rulebook ambiguity (a rulebook-quality issue, not
        // an inter-rulebook conflict). Both are filtered out by the
        // joint per-pair check.
        let mut is_dispute = false;
        'outer: for i in 0..unique.len() {
            for j in (i + 1)..unique.len() {
                if unique[i].source_rulebooks != unique[j].source_rulebooks
                    && unique[i].bindings != unique[j].bindings
                {
                    is_dispute = true;
                    break 'outer;
                }
            }
        }
        if is_dispute {
            out.push(DisputedAnswer {
                query: answer.query.clone(),
                candidates: unique,
                resolution_required: ResolutionRequirement::HumanReview,
            });
        }
    }
    out
}

/// Parallel attribution maps from clause IDs to the rulebook that
/// produced them. Mirrors `adjudication_connector::LoweredKb`'s
/// fact_provenance / rule_provenance fields, lifted to the pipeline
/// layer so downstream consumers don't have to reach into the
/// connector to read attribution.
#[derive(Debug, Clone, Default)]
pub struct ClauseProvenanceTable {
    pub fact_provenance: std::collections::HashMap<logic_engine::FactId, ClauseProvenance>,
    pub rule_provenance: std::collections::HashMap<logic_engine::RuleId, ClauseProvenance>,
}

impl ClauseProvenanceTable {
    fn from_lowered(lowered: &LoweredKb) -> Self {
        Self {
            fact_provenance: lowered.fact_provenance.clone(),
            rule_provenance: lowered.rule_provenance.clone(),
        }
    }
}

// Re-export `ClauseProvenance` and `TrustTier` so callers can
// construct rulebook inputs without depending on
// `adjudication-connector` directly. `LoweredKb` stays internal —
// the pipeline owns the lowering and exposes attribution via
// `ClauseProvenanceTable` only.
pub use adjudication_connector::{ClauseProvenance as RulebookProvenance, TrustTier as RulebookTrustTier};

/// The pipeline's verdict — distinct from the audit trail's
/// `AdjudicationOutcome` so callers can pattern-match without
/// reaching into the trail.
#[derive(Debug)]
pub enum Verdict {
    /// Every gating check passed and the engine returned answers.
    Resolved { answers: Vec<AdjudicationResult> },
    /// At least one gating check (ADJ02 coverage, ADJ03
    /// polarity-modality) failed. The audit trail records the full
    /// violation list; this variant carries a summary count so
    /// callers can branch without parsing the trail.
    Blocked { violation_count: usize },
    /// Lowering or engine execution failed.
    EngineError(String),
}

// ---------------------------------------------------------------------------
// ADJ16 step 4 — Multi-model agreement weighting
// ---------------------------------------------------------------------------

/// Merge multiple rulebook IRs into a single rulebook IR where each
/// rule's weight reflects multi-model agreement.
///
/// The motivation, from
/// [ADJ16](../../../specs/ADJ16-engine-programmatic-adjudication.md)
/// §"Probabilistic extension (ProbLog)" §1: when N independent
/// models elicit rulebooks via
/// `adjudication_rulebook::acquire_rulebook_adversarial`, rules
/// that *all N models produced* are more trustworthy than rules
/// *only one model produced*. Step 4 quantifies that intuition by
/// converting `definitional(head, [body...])` rules to
/// `probabilistic(weight, head, [body...])` rules with
/// `weight = count_of_rulebooks_containing_the_rule / N`.
///
/// Algorithm:
/// 1. For each input rulebook, walk every Rule node.
/// 2. For `definitional(head, [body...])` rules, group by exact
///    Term equality of `(head, body)` across all rulebooks. The
///    weight for each group is `count / total_rulebooks`. The
///    output emits one `probabilistic(weight, head, [body...])`
///    rule per group.
/// 3. `probabilistic(p, head, [body...])` rules pass through
///    unchanged — the caller's existing probability is preserved.
/// 4. `constraint([body...])` and `default(head, [body...],
///    [exceptions...])` rules pass through unchanged. They are not
///    aggregated in v0.7; the agreement-weight idiom is naturally
///    expressed over definitional rules and probabilistic rules,
///    and a future iteration can extend to defaults if needed.
///
/// Edge cases:
/// - Empty `rulebooks` slice returns an empty IRDocument with the
///   given `output_document_id`.
/// - One rulebook returns the same rules with weight 1.0
///   (1 / 1 = 1.0). A `definitional` becomes
///   `probabilistic(1.0, ...)`; nothing else changes.
///
/// The output IR is suitable to feed back into
/// [`run_with_rulebooks`] as a single
/// `(IRDocument, ClauseProvenance)` pair. The provenance ID
/// usually reflects the agreement step (e.g.,
/// `"adversarial-agreement-2026-05-12"`); the trust tier is
/// caller-chosen (`Tentative` if the underlying rulebooks were
/// LLM-elicited; `Reviewed` after expert sign-off).
pub fn compute_agreement_weighted_rulebook(
    rulebooks: &[&IRDocument],
    output_document_id: &str,
) -> IRDocument {
    use logic_core::Term;
    use adjudication_ir::{DocumentId, NodeId, NodeKind, Polarity, Modality, IRNode};

    let doc_id = DocumentId::new(output_document_id);
    let total = rulebooks.len();
    if total == 0 {
        return IRDocument {
            document_id: doc_id,
            nodes: Vec::new(),
            edges: Vec::new(),
        };
    }

    // Group definitional rules by (head, body_list) Term equality.
    // Term doesn't implement Hash/Eq (only PartialEq), so we use a
    // Vec of (term, count) and do linear lookups. Definitional rule
    // counts in any realistic rulebook are small (typically < 20),
    // so the O(n²) is fine.
    let mut definitional_groups: Vec<(Term, usize)> = Vec::new();
    // Passthrough rules preserved in declaration order across all
    // input rulebooks. These keep their existing term as-is.
    let mut passthrough_terms: Vec<Term> = Vec::new();

    for rb in rulebooks {
        // Track per-rulebook membership so a single rulebook can't
        // inflate a group's count by listing the same rule twice.
        let mut seen_in_this_rb: Vec<Term> = Vec::new();
        for node in &rb.nodes {
            if node.kind != NodeKind::Rule {
                continue;
            }
            let term = &node.term;
            let (functor, args) = match term {
                Term::Compound { functor, args } => (functor.as_str(), args),
                _ => {
                    // Not a recognised rule shape — pass through as-is.
                    if !passthrough_terms.iter().any(|t| t == term) {
                        passthrough_terms.push(term.clone());
                    }
                    continue;
                }
            };
            match functor {
                "definitional" if args.len() == 2 => {
                    if seen_in_this_rb.iter().any(|t| t == term) {
                        continue; // skip duplicate within a single rulebook
                    }
                    seen_in_this_rb.push(term.clone());
                    if let Some(entry) = definitional_groups
                        .iter_mut()
                        .find(|(t, _)| t == term)
                    {
                        entry.1 += 1;
                    } else {
                        definitional_groups.push((term.clone(), 1));
                    }
                }
                _ => {
                    if !passthrough_terms.iter().any(|t| t == term) {
                        passthrough_terms.push(term.clone());
                    }
                }
            }
        }
    }

    // Build output nodes: one probabilistic rule per definitional
    // group, then every passthrough rule preserved as-is.
    let mut nodes: Vec<IRNode> = Vec::new();
    let mut next_idx = 0usize;
    for (def_term, count) in definitional_groups {
        let (head, body_list) = match &def_term {
            Term::Compound { args, .. } => (args[0].clone(), args[1].clone()),
            _ => unreachable!("definitional groups only contain compound terms"),
        };
        let weight = count as f64 / total as f64;
        let new_term = logic_core::compound(
            "probabilistic",
            vec![logic_core::float(weight), head, body_list],
        );
        nodes.push(IRNode {
            id: NodeId::new(format!("R{}", next_idx)),
            kind: NodeKind::Rule,
            term: new_term,
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: Vec::new(),
            confidence: 1.0,
            discard_reason: None,
            metadata: Default::default(),
        });
        next_idx += 1;
    }
    for term in passthrough_terms {
        nodes.push(IRNode {
            id: NodeId::new(format!("R{}", next_idx)),
            kind: NodeKind::Rule,
            term,
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: Vec::new(),
            confidence: 1.0,
            discard_reason: None,
            metadata: Default::default(),
        });
        next_idx += 1;
    }

    IRDocument {
        document_id: doc_id,
        nodes,
        edges: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// One-call end-to-end. Runs coverage + polarity/modality + (optional)
/// ADJ04 round-trip, records each into the audit trail, then runs the
/// engine if (and only if) every gating check passed.
///
/// `adjudication_id` and `now()` are caller-supplied because the
/// pipeline is otherwise pure: the same input + the same id + the
/// same timestamps deterministically produces the same audit trail
/// (for the LLM-free passes — ADJ04 records whatever the gateway
/// returned, which is the model's job to make deterministic via
/// `temperature = 0.0`).
///
/// `gateway` controls ADJ04. Passing `None` preserves the v0.2
/// behaviour: ADJ04 is recorded as `Skipped`. Passing `Some(&g)`
/// with `Renderer` + `Nli` clients registered runs the real check
/// and surfaces violations as `Failed` in the audit trail. If a
/// required role is missing, ADJ04 records `Failed` with a single
/// telemetry-only violation describing the configuration gap, and
/// the engine still runs (round-trip is advisory, not gating, at
/// v0.3).
pub fn run<F: Fn() -> String>(
    input: PipelineInput,
    adjudication_id: AdjudicationId,
    now: F,
) -> PipelineOutput {
    run_with_gateway(input, adjudication_id, now, None)
}

/// Same as [`run`] but with an explicit `GatewayConfig`. v0.3's
/// preferred entry point; [`run`] is kept for binary-compat with v0.2
/// callers.
pub fn run_with_gateway<F: Fn() -> String>(
    input: PipelineInput,
    adjudication_id: AdjudicationId,
    now: F,
    gateway: Option<&GatewayConfig>,
) -> PipelineOutput {
    let started_at = now();
    let mut trail = AuditTrail::new(adjudication_id, started_at.clone());

    // ---------- record the input document ----------
    trail.documents.push(Document {
        id: DocumentId::new(input.document.id.clone()),
        name: input.document.name.clone(),
        received_at: input.document.received_at.clone(),
        normalized_text: input.document.normalized_text.clone(),
        normalization: NormalizationRecord {
            pipeline: input.document.normalization_pipeline.clone(),
            version: input.document.normalization_version.clone(),
            options: Default::default(),
        },
        raw_base64: None,
        appended_turns: Vec::new(),
    });

    // ---------- record the IR nodes ----------
    for node in &input.ir_document.nodes {
        trail.ir_nodes.push(ir_node_to_audit(&input.document.id, node));
    }

    // ---------- ADJ02 coverage ----------
    let cov_doc = CovDocument {
        id: input.ir_document.document_id.clone(),
        normalized_text: input.document.normalized_text.clone(),
    };
    let cov_started = now();
    let cov_result = check_coverage(&cov_doc, &input.ir_document);
    let cov_completed = now();
    trail.checker_results.push(coverage_to_checker_result(
        cov_started,
        cov_completed,
        &cov_result,
    ));

    // ---------- ADJ03 polarity/modality ----------
    let pm_started = now();
    let pm_result = check_propagation(&input.ir_document);
    let pm_completed = now();
    trail.checker_results.push(propagation_to_checker_result(
        pm_started,
        pm_completed,
        &pm_result,
    ));

    // ---------- ADJ04 round-trip (gated on a gateway being supplied) ----------
    // We only attempt ADJ04 when the prior gating checks passed —
    // running the LLM on an IR that doesn't even cover its source
    // burns tokens to discover what ADJ02/ADJ03 already told us.
    let prior_gating_ok =
        matches!(cov_result, CoverageResult::Pass) && pm_result.pass();
    let adj04_started = now();
    let adj04_result = if prior_gating_ok {
        run_adj04(gateway, &input.document.normalized_text, &input.ir_document)
    } else {
        Adj04Decision::Skipped
    };
    let adj04_completed = now();
    trail
        .checker_results
        .push(adj04_to_checker_result(adj04_started, adj04_completed, &adj04_result));

    // ---------- ADJ05 adversarial (also gated on prior checks) ----------
    // ADJ05 fires when the gateway has the `Adversary` role
    // registered AND that role's identity differs in `(vendor,
    // model_family)` from `Extractor` (per LM00b independence). Like
    // ADJ04 it's advisory at v0.4 — failures record as Failed but
    // don't block the engine.
    let adj05_started = now();
    let adj05_result = if prior_gating_ok {
        run_adj05(gateway, &input.document.normalized_text, &input.ir_document)
    } else {
        Adj05Decision::Skipped {
            reason: "ADJ02 or ADJ03 failed — adversarial check skipped",
        }
    };
    let adj05_completed = now();
    trail
        .checker_results
        .push(adj05_to_checker_result(adj05_started, adj05_completed, &adj05_result));

    // ---------- gate the engine on coverage + propagation ----------
    let coverage_ok = matches!(cov_result, CoverageResult::Pass);
    let propagation_ok = pm_result.pass();

    if !(coverage_ok && propagation_ok) {
        let violation_count = trail
            .checker_results
            .iter()
            .map(|cr| cr.violations.len())
            .sum();
        trail.outcome = AdjudicationOutcome::ClarificationExhausted {
            unresolved: collect_violations(&trail.checker_results),
        };
        trail.completed_at = Some(now());
        return PipelineOutput {
            verdict: Verdict::Blocked { violation_count },
            audit_trail: trail,
            clause_provenance: None,
            disputed_answers: Vec::new(),
        };
    }

    // ---------- engine ----------
    let answers = match adjudication_connector::run_adjudication(&input.ir_document) {
        Ok(rs) => rs,
        Err(e) => {
            let detail = format!("{e:?}");
            trail.outcome = AdjudicationOutcome::Aborted {
                reason: detail.clone(),
            };
            trail.completed_at = Some(now());
            return PipelineOutput {
                verdict: Verdict::EngineError(detail),
                audit_trail: trail,
                clause_provenance: None,
                disputed_answers: Vec::new(),
            };
        }
    };

    trail.engine_artifacts = Some(EngineArtifacts {
        engine_version: "logic-engine 0.x".to_string(),
        search_mode: SearchMode::AutoDetect,
        kb_summary: KbSummary {
            // v0.1 leaves the KB-summary counts empty — the connector
            // doesn't currently expose them on the result type.
            // A follow-up can plumb fact_count / rule_count through.
            fact_count: 0,
            rule_count: 0,
            fact_ids: Vec::new(),
            rule_ids: Vec::new(),
            all_certain: answers
                .iter()
                .all(|a| !matches!(a.result, logic_engine::SearchResult::EnumerateAllResult { .. })),
        },
        proof_dag: serde_json::Value::Null,
        formula: None,
        wmc_result: None,
        answer: answers_to_audit_json(&answers),
    });
    trail.outcome = AdjudicationOutcome::Resolved {
        answer: answers_to_audit_json(&answers),
    };
    trail.completed_at = Some(now());

    PipelineOutput {
        verdict: Verdict::Resolved { answers },
        audit_trail: trail,
        clause_provenance: None,
        disputed_answers: Vec::new(),
    }
}

/// ADJ16 step 2: the rulebook-merging entry point.
///
/// Same gating checks as [`run_with_gateway`] (ADJ02 coverage, ADJ03
/// polarity/modality, optional ADJ04 round-trip, optional ADJ05
/// adversarial). What changes: at engine time, the input document's
/// IR is lowered with a default `Authoritative` provenance keyed to
/// the document id, and each entry in `rulebooks` is lowered with
/// its caller-supplied provenance. All `LoweredKb`s are combined via
/// [`LoweredKb::extend`] before queries run. The returned
/// `PipelineOutput.clause_provenance` carries the per-FactId /
/// per-RuleId attribution so the audit trail (and ADJ16 step 3's
/// future `DisputedAnswer` resolution) can trace each cited clause
/// back to its origin.
///
/// `rulebooks` is a slice of `(IRDocument, ClauseProvenance)` pairs.
/// Pass an empty slice to mimic [`run_with_gateway`]'s behavior (no
/// rulebooks injected, but the input doc is still lowered with
/// provenance and the attribution table is populated for source
/// facts).
///
/// Queries are extracted only from `input.ir_document`. Any Query
/// nodes in rulebook IR documents are ignored — rulebooks contribute
/// rules and facts, not questions.
pub fn run_with_rulebooks<F: Fn() -> String>(
    input: PipelineInput,
    adjudication_id: AdjudicationId,
    now: F,
    gateway: Option<&GatewayConfig>,
    rulebooks: &[(IRDocument, ClauseProvenance)],
) -> PipelineOutput {
    // Reuse run_with_gateway up to (but not including) the engine
    // step by replaying its logic inline. We need a custom engine
    // call here because `run_adjudication` uses the bare
    // `lower_to_kb` path; we instead build a combined `LoweredKb`
    // via `lower_to_kb_with_provenance`.
    let started_at = now();
    let mut trail = AuditTrail::new(adjudication_id, started_at.clone());

    trail.documents.push(Document {
        id: DocumentId::new(input.document.id.clone()),
        name: input.document.name.clone(),
        received_at: input.document.received_at.clone(),
        normalized_text: input.document.normalized_text.clone(),
        normalization: NormalizationRecord {
            pipeline: input.document.normalization_pipeline.clone(),
            version: input.document.normalization_version.clone(),
            options: Default::default(),
        },
        raw_base64: None,
        appended_turns: Vec::new(),
    });

    for node in &input.ir_document.nodes {
        trail.ir_nodes.push(ir_node_to_audit(&input.document.id, node));
    }

    let cov_doc = CovDocument {
        id: input.ir_document.document_id.clone(),
        normalized_text: input.document.normalized_text.clone(),
    };
    let cov_started = now();
    let cov_result = check_coverage(&cov_doc, &input.ir_document);
    let cov_completed = now();
    trail.checker_results.push(coverage_to_checker_result(
        cov_started,
        cov_completed,
        &cov_result,
    ));

    let pm_started = now();
    let pm_result = check_propagation(&input.ir_document);
    let pm_completed = now();
    trail.checker_results.push(propagation_to_checker_result(
        pm_started,
        pm_completed,
        &pm_result,
    ));

    let prior_gating_ok =
        matches!(cov_result, CoverageResult::Pass) && pm_result.pass();
    let adj04_started = now();
    let adj04_result = if prior_gating_ok {
        run_adj04(gateway, &input.document.normalized_text, &input.ir_document)
    } else {
        Adj04Decision::Skipped
    };
    let adj04_completed = now();
    trail
        .checker_results
        .push(adj04_to_checker_result(adj04_started, adj04_completed, &adj04_result));

    let adj05_started = now();
    let adj05_result = if prior_gating_ok {
        run_adj05(gateway, &input.document.normalized_text, &input.ir_document)
    } else {
        Adj05Decision::Skipped {
            reason: "ADJ02 or ADJ03 failed — adversarial check skipped",
        }
    };
    let adj05_completed = now();
    trail
        .checker_results
        .push(adj05_to_checker_result(adj05_started, adj05_completed, &adj05_result));

    let coverage_ok = matches!(cov_result, CoverageResult::Pass);
    let propagation_ok = pm_result.pass();

    if !(coverage_ok && propagation_ok) {
        let violation_count = trail
            .checker_results
            .iter()
            .map(|cr| cr.violations.len())
            .sum();
        trail.outcome = AdjudicationOutcome::ClarificationExhausted {
            unresolved: collect_violations(&trail.checker_results),
        };
        trail.completed_at = Some(now());
        return PipelineOutput {
            verdict: Verdict::Blocked { violation_count },
            audit_trail: trail,
            clause_provenance: None,
            disputed_answers: Vec::new(),
        };
    }

    // ---------- engine, provenance-aware ----------
    let source_provenance = ClauseProvenance::new(
        input.ir_document.document_id.0.clone(),
        TrustTier::Authoritative,
    );
    let mut combined = match lower_to_kb_with_provenance(
        &input.ir_document,
        source_provenance,
    ) {
        Ok(l) => l,
        Err(e) => {
            let detail = format!("{e:?}");
            trail.outcome = AdjudicationOutcome::Aborted {
                reason: detail.clone(),
            };
            trail.completed_at = Some(now());
            return PipelineOutput {
                verdict: Verdict::EngineError(detail),
                audit_trail: trail,
                clause_provenance: None,
                disputed_answers: Vec::new(),
            };
        }
    };
    for (rb_ir, rb_prov) in rulebooks {
        match lower_to_kb_with_provenance(rb_ir, rb_prov.clone()) {
            Ok(lowered) => combined.extend(lowered),
            Err(e) => {
                let detail = format!(
                    "rulebook lowering failed for {}: {e:?}",
                    rb_prov.source_rulebook_id
                );
                trail.outcome = AdjudicationOutcome::Aborted {
                    reason: detail.clone(),
                };
                trail.completed_at = Some(now());
                return PipelineOutput {
                    verdict: Verdict::EngineError(detail),
                    audit_trail: trail,
                    clause_provenance: None,
                    disputed_answers: Vec::new(),
                };
            }
        }
    }

    // ADJ16 step 3: when rulebooks are attached, default to
    // `EnumerateAll` mode so the engine returns every successful
    // proof. Dispute detection requires multiple proofs per query —
    // `FindFirst` would stop at the first success and hide
    // disagreements. When no rulebooks are attached, fall back to
    // `AutoDetect` (the engine picks based on whether the KB is
    // all-Certain).
    let search_mode = if rulebooks.is_empty() {
        logic_engine::SearchMode::AutoDetect
    } else {
        logic_engine::SearchMode::EnumerateAll
    };
    let queries = adjudication_connector::extract_queries(&input.ir_document);
    let answers: Vec<AdjudicationResult> = queries
        .into_iter()
        .map(|q| {
            let result = logic_engine::search(&q, &combined.kb, search_mode);
            AdjudicationResult { query: q, result }
        })
        .collect();

    let provenance_table = ClauseProvenanceTable::from_lowered(&combined);
    let disputed_answers = detect_disputes(&answers, &provenance_table);

    let audit_search_mode = match search_mode {
        logic_engine::SearchMode::FindFirst => SearchMode::FindFirst,
        logic_engine::SearchMode::EnumerateAll => SearchMode::EnumerateAll,
        logic_engine::SearchMode::AutoDetect => SearchMode::AutoDetect,
    };
    trail.engine_artifacts = Some(EngineArtifacts {
        engine_version: "logic-engine 0.x".to_string(),
        search_mode: audit_search_mode,
        kb_summary: KbSummary {
            fact_count: provenance_table.fact_provenance.len(),
            rule_count: provenance_table.rule_provenance.len(),
            fact_ids: Vec::new(),
            rule_ids: Vec::new(),
            all_certain: answers
                .iter()
                .all(|a| !matches!(a.result, logic_engine::SearchResult::EnumerateAllResult { .. })),
        },
        proof_dag: serde_json::Value::Null,
        formula: None,
        wmc_result: None,
        answer: answers_to_audit_json(&answers),
    });
    trail.outcome = AdjudicationOutcome::Resolved {
        answer: answers_to_audit_json(&answers),
    };
    trail.completed_at = Some(now());

    PipelineOutput {
        verdict: Verdict::Resolved { answers },
        audit_trail: trail,
        clause_provenance: Some(provenance_table),
        disputed_answers,
    }
}

// ---------------------------------------------------------------------------
// Translations from checker types to audit-trail types
// ---------------------------------------------------------------------------

fn ir_node_to_audit(doc_id: &str, node: &IRNode) -> IrNode {
    IrNode {
        id: NodeId::new(node.id.0.clone()),
        document_id: DocumentId::new(doc_id.to_string()),
        payload: serde_json::json!({
            "id": node.id.0,
            "kind": format!("{:?}", node.kind),
            "polarity": format!("{:?}", node.polarity),
            "modality": format!("{:?}", node.modality),
        }),
    }
}

fn coverage_to_checker_result(
    started_at: String,
    completed_at: String,
    result: &CoverageResult,
) -> CheckerResult {
    let (outcome, violations) = match result {
        CoverageResult::Pass => (PassOutcome::Passed, Vec::new()),
        CoverageResult::Fail { violations } => (
            PassOutcome::Failed,
            violations.iter().map(coverage_violation_to_audit).collect(),
        ),
    };
    CheckerResult {
        pass_name: PassName::Adj02Coverage,
        pass_version: "v2.0".to_string(),
        started_at,
        completed_at,
        outcome,
        violations,
        telemetry: Default::default(),
    }
}

fn coverage_violation_to_audit(v: &CoverageViolation) -> Violation {
    use adjudication_ir::SpanLocation;
    let span_loc_node_id = |loc: &SpanLocation| -> NodeId {
        match loc {
            SpanLocation::Node(id) => ir_node_id_to_audit(id),
            SpanLocation::Edge(eid) => NodeId::new(format!("edge:{}", eid.0)),
        }
    };
    let (node_id, detail) = match v {
        CoverageViolation::SpanWrongDocument {
            location, expected, found,
        } => (
            span_loc_node_id(location),
            serde_json::json!({
                "kind": "SpanWrongDocument",
                "expected": &expected.0,
                "found": &found.0,
            }),
        ),
        CoverageViolation::InvalidSpan { location, .. } => (
            span_loc_node_id(location),
            serde_json::json!({ "kind": "InvalidSpan" }),
        ),
        // Catch-all: every other variant gets its Debug rendering. The
        // pipeline keeps the audit-trail JSON open-ended; a follow-up
        // can pattern-match each variant explicitly if a downstream
        // consumer needs structured detail.
        other => (
            // A best-effort id. Coverage violations always carry a
            // node_id; this fallback is for variants the pipeline has
            // not yet specialised.
            NodeId::new(String::new()),
            serde_json::json!({
                "kind": "Other",
                "debug": format!("{other:?}"),
            }),
        ),
    };
    Violation {
        node_id,
        pass_name: PassName::Adj02Coverage,
        kind: ClarificationKind::UncoveredSpan,
        detail,
        triggered_dialogue_turn: None,
        resolved: false,
    }
}

fn propagation_to_checker_result(
    started_at: String,
    completed_at: String,
    result: &PropagationResult,
) -> CheckerResult {
    let outcome = if result.pass() {
        PassOutcome::Passed
    } else {
        PassOutcome::Failed
    };
    let violations: Vec<Violation> = result
        .violations
        .iter()
        .map(propagation_violation_to_audit)
        .collect();
    let mut telemetry = std::collections::BTreeMap::new();
    if !result.warnings.is_empty() {
        telemetry.insert(
            "warning_count".to_string(),
            serde_json::json!(result.warnings.len()),
        );
        telemetry.insert(
            "warnings".to_string(),
            serde_json::Value::Array(
                result
                    .warnings
                    .iter()
                    .map(propagation_warning_to_json)
                    .collect(),
            ),
        );
    }
    CheckerResult {
        pass_name: PassName::Adj03PolarityModality,
        pass_version: "v2.0".to_string(),
        started_at,
        completed_at,
        outcome,
        violations,
        telemetry,
    }
}

fn propagation_violation_to_audit(v: &PropagationViolation) -> Violation {
    let (node_id, kind, detail) = match v {
        PropagationViolation::InheritWithoutParent { node_id, field } => (
            ir_node_id_to_audit(node_id),
            ClarificationKind::InheritChainUnresolved,
            serde_json::json!({
                "kind": "InheritWithoutParent",
                "field": format!("{field:?}"),
            }),
        ),
        PropagationViolation::MultiParentConflict {
            node_id,
            field,
            candidates,
        } => (
            ir_node_id_to_audit(node_id),
            ClarificationKind::AmbiguousPolarity,
            serde_json::json!({
                "kind": "MultiParentConflict",
                "field": format!("{field:?}"),
                "candidates": candidates
                    .iter()
                    .map(|(id, v)| serde_json::json!({ "parent": &id.0, "value": v }))
                    .collect::<Vec<_>>(),
            }),
        ),
        PropagationViolation::RuledOutMustBeAffirmed {
            node_id,
            actual_polarity,
        } => (
            ir_node_id_to_audit(node_id),
            ClarificationKind::AmbiguousPolarity,
            serde_json::json!({
                "kind": "RuledOutMustBeAffirmed",
                "actual_polarity": format!("{actual_polarity:?}"),
            }),
        ),
        PropagationViolation::UpstreamValidationError { kind } => (
            NodeId::new(String::new()),
            ClarificationKind::InheritChainUnresolved,
            serde_json::json!({
                "kind": "UpstreamValidationError",
                "detail": kind,
            }),
        ),
    };
    Violation {
        node_id,
        pass_name: PassName::Adj03PolarityModality,
        kind,
        detail,
        triggered_dialogue_turn: None,
        resolved: false,
    }
}

fn propagation_warning_to_json(w: &PropagationWarning) -> serde_json::Value {
    serde_json::json!({ "debug": format!("{w:?}") })
}

// ---------------------------------------------------------------------------
// ADJ04 wiring
// ---------------------------------------------------------------------------

/// What the pipeline learned from attempting ADJ04 on a given run.
/// Kept separate from `RoundTripResult` so the `Skipped` and
/// `CheckErrored` cases don't need to manufacture an empty
/// `RoundTripResult`.
enum Adj04Decision {
    /// No gateway supplied OR a prior gating check failed — the
    /// pipeline did not attempt the round-trip.
    Skipped,
    /// The checker ran. `result.violations.is_empty()` is the pass
    /// signal.
    Ran(RoundTripResult),
    /// The checker errored before producing a verdict (missing role,
    /// gateway error, primitive validation exhaustion, …). The
    /// pipeline records this as a Failed pass with the error in
    /// telemetry so the audit trail stays complete.
    CheckErrored(String),
}

fn run_adj04(
    gateway: Option<&GatewayConfig>,
    document_text: &str,
    ir_doc: &IRDocument,
) -> Adj04Decision {
    let Some(g) = gateway else {
        return Adj04Decision::Skipped;
    };
    match check_round_trip(document_text, ir_doc, g, &RoundTripOptions::default()) {
        Ok(result) => Adj04Decision::Ran(result),
        Err(e) => Adj04Decision::CheckErrored(round_trip_err_summary(&e)),
    }
}

fn round_trip_err_summary(e: &RoundTripCheckError) -> String {
    // The checker's Display impl already produces a human-friendly
    // message; we just relay it. The trail records this string in
    // telemetry, not in `violations` — a checker error is operator-
    // surface rather than reviewer-surface.
    format!("{e}")
}

fn adj04_to_checker_result(
    started_at: String,
    completed_at: String,
    decision: &Adj04Decision,
) -> CheckerResult {
    match decision {
        Adj04Decision::Skipped => CheckerResult {
            pass_name: PassName::Adj04RoundTrip,
            pass_version: "not-yet-wired".to_string(),
            started_at,
            completed_at,
            outcome: PassOutcome::Skipped,
            violations: Vec::new(),
            telemetry: Default::default(),
        },
        Adj04Decision::Ran(result) => {
            let outcome = if result.pass() {
                PassOutcome::Passed
            } else {
                PassOutcome::Failed
            };
            let violations: Vec<Violation> = result
                .violations
                .iter()
                .map(round_trip_violation_to_audit)
                .collect();
            let mut telemetry = std::collections::BTreeMap::new();
            telemetry.insert(
                "call_count".to_string(),
                serde_json::json!(result.call_records.len()),
            );
            telemetry.insert(
                "primitive_calls".to_string(),
                serde_json::Value::Array(
                    result
                        .call_records
                        .iter()
                        .map(|c| {
                            serde_json::json!({
                                "primitive": c.primitive,
                                "role": c.role,
                                "prompt_version": c.prompt_version,
                                "prompt_hash": c.prompt_hash,
                                "latency_ms": c.latency_ms,
                                "input_tokens": c.usage.input_tokens,
                                "output_tokens": c.usage.output_tokens,
                            })
                        })
                        .collect(),
                ),
            );
            CheckerResult {
                pass_name: PassName::Adj04RoundTrip,
                pass_version: "v1.0".to_string(),
                started_at,
                completed_at,
                outcome,
                violations,
                telemetry,
            }
        }
        Adj04Decision::CheckErrored(detail) => {
            let mut telemetry = std::collections::BTreeMap::new();
            telemetry.insert(
                "check_error".to_string(),
                serde_json::Value::String(detail.clone()),
            );
            CheckerResult {
                pass_name: PassName::Adj04RoundTrip,
                pass_version: "v1.0".to_string(),
                started_at,
                completed_at,
                outcome: PassOutcome::Failed,
                violations: Vec::new(),
                telemetry,
            }
        }
    }
}

fn round_trip_violation_to_audit(v: &RoundTripViolation) -> Violation {
    Violation {
        node_id: NodeId::new(v.node_id.0.clone()),
        pass_name: PassName::Adj04RoundTrip,
        kind: ClarificationKind::RoundTripDrift,
        detail: serde_json::json!({
            "kind": "RoundTripDrift",
            "rendering": v.rendering,
            "source_excerpt": v.source_excerpt,
            "source_to_rendering": v.source_to_rendering,
            "rendering_to_source": v.rendering_to_source,
            "threshold": v.threshold,
        }),
        triggered_dialogue_turn: None,
        resolved: false,
    }
}

// ---------------------------------------------------------------------------
// ADJ05 wiring
// ---------------------------------------------------------------------------

/// ADJ05 verdict shape — same structure as `Adj04Decision`.
enum Adj05Decision {
    /// No gateway OR Adversary role missing OR independence violated
    /// — the pipeline did not attempt the adversarial check.
    Skipped { reason: &'static str },
    Ran(AdversarialResult),
    CheckErrored(String),
}

fn run_adj05(
    gateway: Option<&GatewayConfig>,
    document_text: &str,
    ir_doc: &IRDocument,
) -> Adj05Decision {
    let Some(g) = gateway else {
        return Adj05Decision::Skipped {
            reason: "no GatewayConfig supplied",
        };
    };
    // ADJ05 requires Adversary registered AND
    // (Extractor, Adversary) coming from different model families.
    if g.client(PrimitiveRole::Adversary).is_none() {
        return Adj05Decision::Skipped {
            reason: "no client registered for Role::Adversary",
        };
    }
    if let Err(violation) = g.check_independence() {
        // Same model family for both roles — the adversary would
        // just rubber-stamp the extractor. Record skipped with a
        // diagnostic reason rather than running a misconfigured
        // check.
        // We can't bake the dynamic string into a `'static` reason
        // because the diagnostic depends on the runtime identities;
        // surface it as a CheckErrored telemetry entry instead.
        return Adj05Decision::CheckErrored(format!(
            "ADJ05 independence violated: {violation}"
        ));
    }
    let opts = AdversarialOptions {
        style: llm_primitives::RenderStyle::Plain,
        domain_hint: String::new(),
    };
    match check_adversarial(document_text, ir_doc, g, &opts) {
        Ok(result) => Adj05Decision::Ran(result),
        Err(e) => Adj05Decision::CheckErrored(adversarial_err_summary(&e)),
    }
}

fn adversarial_err_summary(e: &AdversarialCheckError) -> String {
    format!("{e}")
}

fn adj05_to_checker_result(
    started_at: String,
    completed_at: String,
    decision: &Adj05Decision,
) -> CheckerResult {
    match decision {
        Adj05Decision::Skipped { reason } => {
            let mut telemetry = std::collections::BTreeMap::new();
            telemetry.insert(
                "skipped_reason".to_string(),
                serde_json::Value::String((*reason).to_string()),
            );
            CheckerResult {
                pass_name: PassName::Adj05Adversarial,
                pass_version: "not-yet-wired".to_string(),
                started_at,
                completed_at,
                outcome: PassOutcome::Skipped,
                violations: Vec::new(),
                telemetry,
            }
        }
        Adj05Decision::Ran(result) => {
            let outcome = if result.pass() {
                PassOutcome::Passed
            } else {
                PassOutcome::Failed
            };
            let violations: Vec<Violation> = result
                .violations
                .iter()
                .map(adversarial_violation_to_audit)
                .collect();
            let mut telemetry = std::collections::BTreeMap::new();
            telemetry.insert(
                "call_count".to_string(),
                serde_json::json!(result.call_records.len()),
            );
            telemetry.insert(
                "primitive_calls".to_string(),
                serde_json::Value::Array(
                    result
                        .call_records
                        .iter()
                        .map(|c| {
                            serde_json::json!({
                                "primitive": c.primitive,
                                "role": c.role,
                                "prompt_version": c.prompt_version,
                                "prompt_hash": c.prompt_hash,
                                "latency_ms": c.latency_ms,
                                "input_tokens": c.usage.input_tokens,
                                "output_tokens": c.usage.output_tokens,
                            })
                        })
                        .collect(),
                ),
            );
            CheckerResult {
                pass_name: PassName::Adj05Adversarial,
                pass_version: "v1.0".to_string(),
                started_at,
                completed_at,
                outcome,
                violations,
                telemetry,
            }
        }
        Adj05Decision::CheckErrored(detail) => {
            let mut telemetry = std::collections::BTreeMap::new();
            telemetry.insert(
                "check_error".to_string(),
                serde_json::Value::String(detail.clone()),
            );
            CheckerResult {
                pass_name: PassName::Adj05Adversarial,
                pass_version: "v1.0".to_string(),
                started_at,
                completed_at,
                outcome: PassOutcome::Failed,
                violations: Vec::new(),
                telemetry,
            }
        }
    }
}

fn adversarial_violation_to_audit(v: &AdversarialViolation) -> Violation {
    Violation {
        node_id: NodeId::new(v.node_id.0.clone()),
        pass_name: PassName::Adj05Adversarial,
        kind: ClarificationKind::AdversarialReading,
        detail: serde_json::json!({
            "kind": "AdversarialReading",
            "ir_rendered": v.ir_rendered,
            "adversary_reading": v.adversary_reading,
            "adversary_explanation": v.adversary_explanation,
            "judge_reason": v.judge_reason,
        }),
        triggered_dialogue_turn: None,
        resolved: false,
    }
}

fn collect_violations(checker_results: &[CheckerResult]) -> Vec<Violation> {
    checker_results
        .iter()
        .flat_map(|cr| cr.violations.iter().cloned())
        .collect()
}

fn ir_node_id_to_audit(id: &IRNodeId) -> NodeId {
    NodeId::new(id.0.clone())
}

fn answers_to_audit_json(answers: &[AdjudicationResult]) -> serde_json::Value {
    let arr: Vec<serde_json::Value> = answers
        .iter()
        .map(|a| {
            serde_json::json!({
                "query": format!("{:?}", a.query),
                "result": format!("{:?}", a.result),
            })
        })
        .collect();
    serde_json::Value::Array(arr)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use adjudication_ir::{
        DocumentId as IRDocumentId, IRNode, Modality, NodeId as IRNodeId, NodeKind, Polarity, Span,
    };
    use logic_core::Term;

    fn pipeline_doc() -> PipelineDocument {
        PipelineDocument {
            id: "doc1".into(),
            name: "tsa_declaration".into(),
            received_at: "2026-05-11T08:00:00Z".into(),
            normalized_text: "1 carry-on bag, 1 personal item.".into(),
            normalization_pipeline: "plain-text-v1".into(),
            normalization_version: "1.0.0".into(),
        }
    }

    fn make_ir(nodes: Vec<IRNode>) -> IRDocument {
        IRDocument {
            document_id: IRDocumentId::new("doc1"),
            nodes,
            edges: Vec::new(),
        }
    }

    fn fact_node(id: &str, term: Term, start: usize, end: usize) -> IRNode {
        IRNode {
            id: IRNodeId::new(id.to_string()),
            kind: NodeKind::Fact,
            term,
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![Span::new(IRDocumentId::new("doc1"), start, end)],
            confidence: 1.0,
            discard_reason: None,
            metadata: Default::default(),
        }
    }

    fn make_clock() -> impl Fn() -> String {
        let tick = std::cell::Cell::new(0u32);
        move || {
            let t = tick.get();
            tick.set(t + 1);
            format!("2026-05-11T08:00:0{}Z", t.min(9))
        }
    }

    #[test]
    fn empty_ir_with_empty_text_passes_through_and_resolves() {
        // Smallest possible pipeline run: zero IR nodes, zero text.
        // Coverage is vacuously OK (no spans to validate), propagation
        // is vacuously OK (no nodes to propagate over), engine has no
        // queries, so the verdict is `Resolved` with zero answers.
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: String::new(),
                ..pipeline_doc()
            },
            ir_document: make_ir(Vec::new()),
        };
        let out = run(input, AdjudicationId::new("adj-empty"), make_clock());
        match out.verdict {
            Verdict::Resolved { answers } => assert!(answers.is_empty()),
            other => panic!("expected Resolved, got {other:?}"),
        }
        assert_eq!(out.audit_trail.adjudication_id.0, "adj-empty");
        assert_eq!(out.audit_trail.checker_results.len(), 4);
        // ADJ02 + ADJ03 passed; ADJ04 + ADJ05 are recorded as Skipped.
        assert!(matches!(
            out.audit_trail.checker_results[0].outcome,
            PassOutcome::Passed
        ));
        assert!(matches!(
            out.audit_trail.checker_results[1].outcome,
            PassOutcome::Passed
        ));
        assert!(matches!(
            out.audit_trail.checker_results[2].outcome,
            PassOutcome::Skipped
        ));
        assert!(matches!(
            out.audit_trail.checker_results[3].outcome,
            PassOutcome::Skipped
        ));
        assert!(out.audit_trail.completed_at.is_some());
        assert!(matches!(
            out.audit_trail.outcome,
            AdjudicationOutcome::Resolved { .. }
        ));
    }

    #[test]
    fn coverage_violation_blocks_engine_and_records_full_audit_trail() {
        // The IR cites a span outside the document text — coverage
        // fails, the pipeline reports Blocked, engine never runs.
        let node = fact_node(
            "n1",
            logic_core::atom("anomaly"),
            100,
            150, // way past the 5-char document text
        );
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: "hello".into(),
                ..pipeline_doc()
            },
            ir_document: make_ir(vec![node]),
        };
        let out = run(input, AdjudicationId::new("adj-blocked"), make_clock());
        match out.verdict {
            Verdict::Blocked { violation_count } => assert!(violation_count > 0),
            other => panic!("expected Blocked, got {other:?}"),
        }
        // Audit trail should still be fully populated.
        assert_eq!(out.audit_trail.documents.len(), 1);
        assert_eq!(out.audit_trail.ir_nodes.len(), 1);
        // Coverage checker failed; ADJ03 still ran (we record both
        // even on early-exit, so the trail captures the full state).
        let cov = &out.audit_trail.checker_results[0];
        assert_eq!(cov.pass_name, PassName::Adj02Coverage);
        assert!(matches!(cov.outcome, PassOutcome::Failed));
        assert!(!cov.violations.is_empty());
        // Outcome is ClarificationExhausted, not Resolved.
        assert!(matches!(
            out.audit_trail.outcome,
            AdjudicationOutcome::ClarificationExhausted { .. }
        ));
        // Engine artifacts must NOT be populated.
        assert!(out.audit_trail.engine_artifacts.is_none());
    }

    #[test]
    fn audit_trail_records_input_document_with_normalization_metadata() {
        let input = PipelineInput {
            document: pipeline_doc(),
            ir_document: make_ir(Vec::new()),
        };
        let out = run(input, AdjudicationId::new("adj-doc-meta"), make_clock());
        let d = &out.audit_trail.documents[0];
        assert_eq!(d.id.0, "doc1");
        assert_eq!(d.name, "tsa_declaration");
        assert_eq!(d.normalization.pipeline, "plain-text-v1");
        assert_eq!(d.normalization.version, "1.0.0");
    }

    #[test]
    fn schema_version_is_recorded_on_audit_trail() {
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: String::new(),
                ..pipeline_doc()
            },
            ir_document: make_ir(Vec::new()),
        };
        let out = run(input, AdjudicationId::new("adj-schema"), make_clock());
        assert_eq!(out.audit_trail.schema_version, "ADJ07-v1");
    }

    #[test]
    fn ir_nodes_are_mirrored_into_audit_trail() {
        let n1 = fact_node("n1", logic_core::atom("a"), 0, 1);
        let n2 = fact_node("n2", logic_core::atom("b"), 2, 3);
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: "abc".into(),
                ..pipeline_doc()
            },
            ir_document: make_ir(vec![n1, n2]),
        };
        let out = run(input, AdjudicationId::new("adj-mirror"), make_clock());
        assert_eq!(out.audit_trail.ir_nodes.len(), 2);
        assert_eq!(out.audit_trail.ir_nodes[0].id.0, "n1");
        assert_eq!(out.audit_trail.ir_nodes[1].id.0, "n2");
        // The payload carries kind/polarity/modality stringified — v0.2
        // will store the typed adjudication_ir::IRNode once that crate
        // ships serde derives.
        assert_eq!(out.audit_trail.ir_nodes[0].payload["kind"], "Fact");
    }

    #[test]
    fn checker_pass_versions_are_recorded() {
        // Smoke-check: every checker_result must carry a non-empty
        // pass_version. Replay needs this to know which checker
        // version was used.
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: String::new(),
                ..pipeline_doc()
            },
            ir_document: make_ir(Vec::new()),
        };
        let out = run(input, AdjudicationId::new("adj-versions"), make_clock());
        for cr in &out.audit_trail.checker_results {
            assert!(
                !cr.pass_version.is_empty(),
                "{:?} has empty version",
                cr.pass_name
            );
        }
    }

    #[test]
    fn audit_trail_round_trips_through_serde_json() {
        let n = fact_node("n1", logic_core::atom("hello"), 0, 5);
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: "hello".into(),
                ..pipeline_doc()
            },
            ir_document: make_ir(vec![n]),
        };
        let out = run(input, AdjudicationId::new("adj-json"), make_clock());
        let json = serde_json::to_string(&out.audit_trail).expect("AuditTrail serializes");
        let back: AuditTrail =
            serde_json::from_str(&json).expect("AuditTrail deserializes");
        assert_eq!(back, out.audit_trail);
    }

    // -----------------------------------------------------------------
    // ADJ04 gateway-wired tests
    // -----------------------------------------------------------------
    //
    // These tests use scripted LLM clients (one for `Renderer`, one
    // for `Nli`) so the pipeline can exercise the real `check_round_trip`
    // path without needing a live model. The pattern mirrors the
    // scripted clients used inside `adjudication-round-trip`; we keep
    // the two crates' fixtures separate so a future refactor (e.g.,
    // a shared `llm-test-utils` crate) is a local change.

    use llm_gateway::{
        Capabilities, CompletionJsonResponse, CompletionRequest, CompletionResponse,
        FinishReason as LlmFinishReason, JsonSchema, LlmClient, LlmError, ProviderIdentity,
        TokenUsage,
    };
    use llm_primitives::{GatewayConfig, Role};
    use std::sync::Mutex;

    fn renderer_id() -> ProviderIdentity {
        ProviderIdentity {
            vendor: "mock".into(),
            model_family: "haiku-renderer".into(),
            model_version: "1".into(),
            endpoint: None,
        }
    }

    fn nli_id() -> ProviderIdentity {
        ProviderIdentity {
            vendor: "mock".into(),
            model_family: "nli-debertav3".into(),
            model_version: "1".into(),
            endpoint: None,
        }
    }

    struct ScriptedRenderer {
        texts: Mutex<Vec<String>>,
    }
    impl ScriptedRenderer {
        fn new(texts: Vec<&str>) -> Self {
            Self {
                texts: Mutex::new(texts.into_iter().rev().map(String::from).collect()),
            }
        }
    }
    impl LlmClient for ScriptedRenderer {
        fn identity(&self) -> ProviderIdentity {
            renderer_id()
        }
        fn capabilities(&self) -> Capabilities {
            Capabilities::modern_frontier()
        }
        fn complete(&self, _r: CompletionRequest) -> Result<CompletionResponse, LlmError> {
            let text = self.texts.lock().unwrap().pop().expect("renderer drained");
            Ok(CompletionResponse {
                text,
                model: "haiku-renderer".into(),
                usage: TokenUsage::default(),
                finish_reason: LlmFinishReason::Stop,
                provider_id: renderer_id(),
                latency_ms: 1,
            })
        }
        fn complete_json(
            &self,
            _r: CompletionRequest,
            _s: &JsonSchema,
        ) -> Result<CompletionJsonResponse, LlmError> {
            unreachable!("render_node uses complete")
        }
    }

    struct ScriptedNli {
        scripts: Mutex<Vec<(bool, f32, bool, f32)>>,
    }
    impl ScriptedNli {
        fn new(s: Vec<(bool, f32, bool, f32)>) -> Self {
            Self {
                scripts: Mutex::new(s.into_iter().rev().collect()),
            }
        }
    }
    impl LlmClient for ScriptedNli {
        fn identity(&self) -> ProviderIdentity {
            nli_id()
        }
        fn capabilities(&self) -> Capabilities {
            Capabilities::modern_frontier()
        }
        fn complete(&self, _r: CompletionRequest) -> Result<CompletionResponse, LlmError> {
            unreachable!("entail uses complete_json")
        }
        fn complete_json(
            &self,
            _r: CompletionRequest,
            _s: &JsonSchema,
        ) -> Result<CompletionJsonResponse, LlmError> {
            let (p_h, p_s, h_p, h_s) = self.scripts.lock().unwrap().pop().expect("nli drained");
            let parsed = serde_json::json!({
                "premise_entails_hypothesis": p_h,
                "p_to_h_score": p_s,
                "hypothesis_entails_premise": h_p,
                "h_to_p_score": h_s,
            });
            Ok(CompletionJsonResponse {
                raw_text: parsed.to_string(),
                parsed,
                schema_valid: true,
                model: "nli-debertav3".into(),
                usage: TokenUsage::default(),
                provider_id: nli_id(),
                latency_ms: 1,
                polyfill_used: false,
            })
        }
    }

    fn gateway_with_scripted(
        renderings: Vec<&str>,
        entailments: Vec<(bool, f32, bool, f32)>,
    ) -> GatewayConfig {
        GatewayConfig::new()
            .with_client(Role::Renderer, Box::new(ScriptedRenderer::new(renderings)))
            .with_client(Role::Nli, Box::new(ScriptedNli::new(entailments)))
    }

    #[test]
    fn adj04_runs_passed_with_high_scores_under_gateway() {
        let n = fact_node("n1", logic_core::atom("hello"), 0, 5);
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: "hello".into(),
                ..pipeline_doc()
            },
            ir_document: make_ir(vec![n]),
        };
        let g = gateway_with_scripted(vec!["passenger said hello"], vec![(true, 0.95, true, 0.92)]);
        let out = run_with_gateway(input, AdjudicationId::new("adj-rt-pass"), make_clock(), Some(&g));
        let adj04 = &out.audit_trail.checker_results[2];
        assert_eq!(adj04.pass_name, PassName::Adj04RoundTrip);
        assert_eq!(adj04.pass_version, "v1.0");
        assert!(matches!(adj04.outcome, PassOutcome::Passed));
        assert!(adj04.violations.is_empty());
        // Telemetry should mention the calls (1 render + 1 entail = 2).
        assert_eq!(adj04.telemetry["call_count"], 2);
        // Verdict still Resolved (engine still runs; ADJ04 is advisory).
        assert!(matches!(out.verdict, Verdict::Resolved { .. }));
    }

    #[test]
    fn adj04_runs_failed_with_drift_under_gateway() {
        let n = fact_node("n1", logic_core::atom("hello"), 0, 5);
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: "hello".into(),
                ..pipeline_doc()
            },
            ir_document: make_ir(vec![n]),
        };
        // Source-to-rendering score way below the 0.6 default threshold.
        let g = gateway_with_scripted(
            vec!["passenger admitted to smuggling contraband"],
            vec![(false, 0.10, true, 0.90)],
        );
        let out = run_with_gateway(input, AdjudicationId::new("adj-rt-drift"), make_clock(), Some(&g));
        let adj04 = &out.audit_trail.checker_results[2];
        assert!(matches!(adj04.outcome, PassOutcome::Failed));
        assert_eq!(adj04.violations.len(), 1);
        assert_eq!(adj04.violations[0].pass_name, PassName::Adj04RoundTrip);
        assert_eq!(adj04.violations[0].kind, ClarificationKind::RoundTripDrift);
        // ADJ04 is *advisory* at v0.3 — engine still runs.
        assert!(matches!(out.verdict, Verdict::Resolved { .. }));
        assert!(out.audit_trail.engine_artifacts.is_some());
    }

    #[test]
    fn adj04_records_skipped_when_no_gateway_provided() {
        // The plain `run` entry point passes `None` for the gateway —
        // ADJ04 must record as Skipped exactly as in v0.2.
        let n = fact_node("n1", logic_core::atom("hello"), 0, 5);
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: "hello".into(),
                ..pipeline_doc()
            },
            ir_document: make_ir(vec![n]),
        };
        let out = run(input, AdjudicationId::new("adj-no-gw"), make_clock());
        let adj04 = &out.audit_trail.checker_results[2];
        assert!(matches!(adj04.outcome, PassOutcome::Skipped));
        assert_eq!(adj04.pass_version, "not-yet-wired");
    }

    #[test]
    fn adj04_records_failed_when_required_role_missing_from_gateway() {
        // A gateway is supplied but the `Renderer` role isn't registered —
        // the round-trip checker surfaces `PrimitiveError::NoClientForRole`,
        // which the pipeline records as Failed with the error in telemetry.
        let n = fact_node("n1", logic_core::atom("hello"), 0, 5);
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: "hello".into(),
                ..pipeline_doc()
            },
            ir_document: make_ir(vec![n]),
        };
        let g = GatewayConfig::new(); // empty
        let out = run_with_gateway(input, AdjudicationId::new("adj-rt-noclient"), make_clock(), Some(&g));
        let adj04 = &out.audit_trail.checker_results[2];
        assert!(matches!(adj04.outcome, PassOutcome::Failed));
        let detail = adj04.telemetry["check_error"].as_str().unwrap();
        assert!(detail.contains("renderer") || detail.contains("Renderer"));
    }

    #[test]
    fn adj04_is_skipped_when_prior_gating_failed_even_if_gateway_supplied() {
        // Coverage fails → don't waste LLM tokens on ADJ04.
        let n = fact_node("n1", logic_core::atom("anomaly"), 100, 150);
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: "hello".into(),
                ..pipeline_doc()
            },
            ir_document: make_ir(vec![n]),
        };
        let g = gateway_with_scripted(vec!["unused"], vec![(true, 0.99, true, 0.99)]);
        let out = run_with_gateway(input, AdjudicationId::new("adj-rt-skip-on-fail"), make_clock(), Some(&g));
        let adj04 = &out.audit_trail.checker_results[2];
        assert!(matches!(adj04.outcome, PassOutcome::Skipped));
        // And the pipeline still Blocks due to the coverage failure.
        assert!(matches!(out.verdict, Verdict::Blocked { .. }));
    }

    // ----- ADJ05 adversarial wiring tests -----

    #[test]
    fn adj05_records_skipped_when_no_gateway_provided() {
        // Plain run() path → no gateway → ADJ05 must Skip.
        let n = fact_node("n1", logic_core::atom("hello"), 0, 5);
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: "hello".into(),
                ..pipeline_doc()
            },
            ir_document: make_ir(vec![n]),
        };
        let out = run(input, AdjudicationId::new("adj-adv-no-gw"), make_clock());
        let adj05 = &out.audit_trail.checker_results[3];
        assert_eq!(adj05.pass_name, PassName::Adj05Adversarial);
        assert!(matches!(adj05.outcome, PassOutcome::Skipped));
    }

    #[test]
    fn adj05_records_skipped_with_reason_when_adversary_role_missing() {
        // Gateway exists but no Adversary role registered.
        let n = fact_node("n1", logic_core::atom("hello"), 0, 5);
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: "hello".into(),
                ..pipeline_doc()
            },
            ir_document: make_ir(vec![n]),
        };
        let g = gateway_with_scripted(vec!["x"], vec![(true, 0.95, true, 0.92)]);
        let out = run_with_gateway(input, AdjudicationId::new("adj-adv-no-role"), make_clock(), Some(&g));
        let adj05 = &out.audit_trail.checker_results[3];
        assert!(matches!(adj05.outcome, PassOutcome::Skipped));
        let reason = adj05.telemetry["skipped_reason"].as_str().unwrap();
        assert!(reason.contains("Adversary") || reason.contains("adversary"));
    }

    // -----------------------------------------------------------------
    // ADJ16 step 2 — run_with_rulebooks tests
    // -----------------------------------------------------------------

    fn query_node(id: &str, term: Term) -> IRNode {
        // Query nodes are synthesized at adjudication time, not
        // extracted from the source. Per ADJ01 (and the convention
        // in adjudication-tsa-demo), they carry empty source_spans
        // so they don't participate in coverage.
        IRNode {
            id: IRNodeId::new(id.to_string()),
            kind: NodeKind::Query,
            term,
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: Vec::new(),
            confidence: 1.0,
            discard_reason: None,
            metadata: Default::default(),
        }
    }

    fn rule_node_no_spans(id: &str, term: Term) -> IRNode {
        // Rule nodes from rulebooks aren't anchored in the source
        // document's text — they come from the rulebook IR. For
        // pipeline tests we use empty source_spans so the source
        // document's coverage check stays clean.
        IRNode {
            id: IRNodeId::new(id.to_string()),
            kind: NodeKind::Rule,
            term,
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: Vec::new(),
            confidence: 1.0,
            discard_reason: None,
            metadata: Default::default(),
        }
    }

    fn rulebook_ir(doc_id: &str, nodes: Vec<IRNode>) -> IRDocument {
        IRDocument {
            document_id: IRDocumentId::new(doc_id),
            nodes,
            edges: Vec::new(),
        }
    }

    fn rule_node(id: &str, term: Term) -> IRNode {
        // Rule nodes from rulebooks don't anchor to the source
        // document; use empty source_spans so the source's coverage
        // check isn't affected by rulebook rules.
        rule_node_no_spans(id, term)
    }

    #[test]
    fn run_with_rulebooks_empty_slice_matches_run_with_gateway_outcome() {
        // No rulebooks attached: behaviour should match the existing
        // run_with_gateway path (modulo provenance metadata on the source).
        let text = "ok";
        let f = fact_node("F1", logic_core::atom("ok"), 0, text.len());
        let q = query_node("Q1", logic_core::atom("ok"));
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: text.into(),
                ..pipeline_doc()
            },
            ir_document: make_ir(vec![f, q]),
        };
        let out = run_with_rulebooks(
            input,
            AdjudicationId::new("adj-rb-empty"),
            make_clock(),
            None,
            &[],
        );
        match &out.verdict {
            Verdict::Resolved { answers } => assert_eq!(answers.len(), 1),
            other => panic!("expected Resolved, got {other:?}"),
        }
        let table = out
            .clause_provenance
            .as_ref()
            .expect("provenance table should be populated by run_with_rulebooks");
        // Source fact gets the document's id as its rulebook id.
        assert_eq!(table.fact_provenance.len(), 1);
        let prov = table.fact_provenance.values().next().unwrap();
        assert_eq!(prov.source_rulebook_id, "doc1");
        assert_eq!(prov.trust_tier, TrustTier::Authoritative);
    }

    #[test]
    fn run_with_rulebooks_merges_external_rule_into_kb() {
        // Source asserts `prohibited(matches)`; rulebook supplies
        // the bridging rule that says any prohibited item makes the
        // declaration non-compliant. Query: is the declaration
        // non-compliant?
        use logic_core::{compound, atom};
        // 21-byte source text; fact spans 0..21 covers all bytes.
        let text = "carry-on bag, matches";
        let source_facts = vec![
            fact_node("F1", compound("prohibited", vec![atom("matches")]), 0, text.len()),
            query_node("Q1", atom("non_compliant")),
        ];
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: text.into(),
                ..pipeline_doc()
            },
            ir_document: make_ir(source_facts),
        };
        // Rulebook: definitional(non_compliant, [prohibited(matches)])
        let rule_term = compound(
            "definitional",
            vec![
                atom("non_compliant"),
                logic_core::logic_list(vec![compound("prohibited", vec![atom("matches")])]),
            ],
        );
        let rb = rulebook_ir("rb-tsa-v1", vec![rule_node("R1", rule_term)]);
        let rb_prov = ClauseProvenance::new("rb-tsa-v1", TrustTier::Reviewed);
        let out = run_with_rulebooks(
            input,
            AdjudicationId::new("adj-rb-merge"),
            make_clock(),
            None,
            &[(rb, rb_prov)],
        );
        match &out.verdict {
            Verdict::Resolved { answers } => {
                assert_eq!(answers.len(), 1);
                // ADJ16 step 3 switches search mode to EnumerateAll
                // when rulebooks are attached, so we now expect a
                // proof-DAG result with at least one proof rather
                // than a single FindFirst binding.
                match &answers[0].result {
                    logic_engine::SearchResult::EnumerateAllResult { dag, .. } => {
                        assert!(
                            !dag.proofs.is_empty(),
                            "expected non_compliant to have at least one proof"
                        );
                    }
                    other => panic!(
                        "expected non_compliant to be provable from merged KB (EnumerateAll), got {:?}",
                        other
                    ),
                }
            }
            other => panic!("expected Resolved, got {other:?}"),
        }
        let table = out.clause_provenance.as_ref().expect("provenance");
        // Source contributes one fact (the prohibited item); rulebook
        // contributes one rule (the bridging definitional). Query is
        // not lowered.
        assert_eq!(table.fact_provenance.len(), 1);
        assert_eq!(table.rule_provenance.len(), 1);
        let rule_prov = table.rule_provenance.values().next().unwrap();
        assert_eq!(rule_prov.source_rulebook_id, "rb-tsa-v1");
        assert_eq!(rule_prov.trust_tier, TrustTier::Reviewed);
    }

    #[test]
    fn run_with_rulebooks_attributes_multiple_rulebooks_distinctly() {
        // Two rulebooks contribute one rule each. After the run, each
        // rule should be attributable to its origin rulebook.
        use logic_core::{compound, atom};
        // Empty text + only a query node = vacuous coverage pass.
        let q = query_node("Q1", atom("any_answer"));
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: String::new(),
                ..pipeline_doc()
            },
            ir_document: make_ir(vec![q]),
        };
        let rule_a = rule_node(
            "Ra",
            compound("definitional", vec![atom("from_a"), logic_core::logic_list(vec![])]),
        );
        let rule_b = rule_node(
            "Rb",
            compound("definitional", vec![atom("from_b"), logic_core::logic_list(vec![])]),
        );
        let rb_a = rulebook_ir("rb-alpha", vec![rule_a]);
        let rb_b = rulebook_ir("rb-beta", vec![rule_b]);
        let out = run_with_rulebooks(
            input,
            AdjudicationId::new("adj-rb-multi"),
            make_clock(),
            None,
            &[
                (rb_a, ClauseProvenance::new("rb-alpha", TrustTier::Tentative)),
                (rb_b, ClauseProvenance::new("rb-beta", TrustTier::Reviewed)),
            ],
        );
        let table = out.clause_provenance.as_ref().expect("provenance");
        assert_eq!(table.rule_provenance.len(), 2);
        let mut origins: Vec<&str> = table
            .rule_provenance
            .values()
            .map(|p| p.source_rulebook_id.as_str())
            .collect();
        origins.sort();
        assert_eq!(origins, vec!["rb-alpha", "rb-beta"]);
        // Tiers preserved per-rulebook.
        let alpha_tier = table
            .rule_provenance
            .values()
            .find(|p| p.source_rulebook_id == "rb-alpha")
            .unwrap()
            .trust_tier;
        let beta_tier = table
            .rule_provenance
            .values()
            .find(|p| p.source_rulebook_id == "rb-beta")
            .unwrap()
            .trust_tier;
        assert_eq!(alpha_tier, TrustTier::Tentative);
        assert_eq!(beta_tier, TrustTier::Reviewed);
    }

    #[test]
    fn run_with_rulebooks_blocks_engine_when_coverage_fails() {
        // Coverage failure short-circuits before the engine runs;
        // clause_provenance stays None because no lowering occurred.
        let n = fact_node("n1", logic_core::atom("anomaly"), 100, 150);
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: "short".into(),
                ..pipeline_doc()
            },
            ir_document: make_ir(vec![n]),
        };
        let out = run_with_rulebooks(
            input,
            AdjudicationId::new("adj-rb-blocked"),
            make_clock(),
            None,
            &[],
        );
        assert!(matches!(out.verdict, Verdict::Blocked { .. }));
        assert!(out.clause_provenance.is_none());
    }

    #[test]
    fn run_with_rulebooks_surfaces_lowering_error_with_rulebook_id() {
        // Malformed rulebook rule should produce an EngineError whose
        // message names the offending rulebook id so the caller can
        // identify which source rulebook to fix.
        use logic_core::{compound, atom};
        let q = query_node("Q1", atom("x"));
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: String::new(),
                ..pipeline_doc()
            },
            ir_document: make_ir(vec![q]),
        };
        // `unknownify` is not a recognised Rule subtype.
        let bad_rule = rule_node("Rbad", compound("unknownify", vec![atom("x")]));
        let rb = rulebook_ir("rb-broken", vec![bad_rule]);
        let out = run_with_rulebooks(
            input,
            AdjudicationId::new("adj-rb-err"),
            make_clock(),
            None,
            &[(rb, ClauseProvenance::new("rb-broken", TrustTier::Tentative))],
        );
        match &out.verdict {
            Verdict::EngineError(msg) => {
                assert!(
                    msg.contains("rb-broken"),
                    "engine error should name the rulebook id, got: {msg}"
                );
            }
            other => panic!("expected EngineError, got {other:?}"),
        }
    }

    #[test]
    fn run_with_rulebooks_ignores_query_nodes_in_rulebook_ir() {
        // A rulebook IR with a Query node should NOT cause that
        // query to be answered. Only the source IR's queries run.
        use logic_core::atom;
        let source_q = query_node("Q1", atom("source_query"));
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: String::new(),
                ..pipeline_doc()
            },
            ir_document: make_ir(vec![source_q]),
        };
        // Rulebook has its own (spurious) query; should be ignored.
        let rb_q = query_node("RQ1", atom("rulebook_query"));
        let rb = rulebook_ir("rb-with-query", vec![rb_q]);
        let out = run_with_rulebooks(
            input,
            AdjudicationId::new("adj-rb-ignore-q"),
            make_clock(),
            None,
            &[(rb, ClauseProvenance::new("rb-with-query", TrustTier::Tentative))],
        );
        match &out.verdict {
            Verdict::Resolved { answers } => {
                assert_eq!(answers.len(), 1, "only the source query should run");
                // The single answer is for `source_query`, not `rulebook_query`.
                assert_eq!(format!("{:?}", answers[0].query), format!("{:?}", atom("source_query")));
            }
            other => panic!("expected Resolved, got {other:?}"),
        }
    }

    #[test]
    fn run_with_gateway_leaves_provenance_table_unpopulated() {
        // Backward compatibility: the existing entry point's output
        // has clause_provenance = None. Callers that don't migrate to
        // run_with_rulebooks see no behavioural change.
        let text = "ok";
        let f = fact_node("F1", logic_core::atom("ok"), 0, text.len());
        let q = query_node("Q1", logic_core::atom("ok"));
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: text.into(),
                ..pipeline_doc()
            },
            ir_document: make_ir(vec![f, q]),
        };
        let out = run(input, AdjudicationId::new("adj-bc"), make_clock());
        assert!(out.clause_provenance.is_none());
        match &out.verdict {
            Verdict::Resolved { answers } => assert_eq!(answers.len(), 1),
            other => panic!("expected Resolved, got {other:?}"),
        }
        // ADJ16 step 3: legacy entry points produce no dispute records.
        assert!(out.disputed_answers.is_empty());
    }

    // -----------------------------------------------------------------
    // ADJ16 step 3 — DisputedAnswer tests
    // -----------------------------------------------------------------

    #[test]
    fn no_dispute_when_single_proof_returned() {
        // Source fact + one rulebook with a single bridging rule.
        // Engine returns exactly one proof; no dispute possible.
        use logic_core::{atom, compound};
        let text = "x";
        let source = vec![
            fact_node("F1", atom("a"), 0, text.len()),
            query_node("Q1", atom("b")),
        ];
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: text.into(),
                ..pipeline_doc()
            },
            ir_document: make_ir(source),
        };
        let bridge = compound(
            "definitional",
            vec![atom("b"), logic_core::logic_list(vec![atom("a")])],
        );
        let rb = rulebook_ir("rb-only", vec![rule_node("R1", bridge)]);
        let out = run_with_rulebooks(
            input,
            AdjudicationId::new("adj-single-proof"),
            make_clock(),
            None,
            &[(rb, ClauseProvenance::new("rb-only", TrustTier::Reviewed))],
        );
        // Resolved with one answer, one proof, no dispute.
        match &out.verdict {
            Verdict::Resolved { answers } => assert_eq!(answers.len(), 1),
            other => panic!("expected Resolved, got {other:?}"),
        }
        assert!(out.disputed_answers.is_empty());
    }

    #[test]
    fn no_dispute_when_two_rulebooks_corroborate_with_same_bindings() {
        // Two rulebooks supply rules that BOTH conclude `b`. Same
        // bindings (empty — `b` is a ground atom), different proof
        // attributions. Per the dispute rule "distinct rulebook sets
        // AND distinct bindings", this is corroboration, not dispute.
        use logic_core::{atom, compound};
        let text = "x";
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: text.into(),
                ..pipeline_doc()
            },
            ir_document: make_ir(vec![
                fact_node("F1", atom("a"), 0, text.len()),
                query_node("Q1", atom("b")),
            ]),
        };
        let rule_a = rule_node(
            "Ra",
            compound("definitional", vec![atom("b"), logic_core::logic_list(vec![atom("a")])]),
        );
        let rule_b = rule_node(
            "Rb",
            compound("definitional", vec![atom("b"), logic_core::logic_list(vec![atom("a")])]),
        );
        let rb_a = rulebook_ir("rb-alpha", vec![rule_a]);
        let rb_b = rulebook_ir("rb-beta", vec![rule_b]);
        let out = run_with_rulebooks(
            input,
            AdjudicationId::new("adj-corroborate"),
            make_clock(),
            None,
            &[
                (rb_a, ClauseProvenance::new("rb-alpha", TrustTier::Tentative)),
                (rb_b, ClauseProvenance::new("rb-beta", TrustTier::Tentative)),
            ],
        );
        // Engine should find two proofs (one via each rule), both
        // producing the same empty binding. Per the dispute
        // semantics, identical bindings = corroboration, not dispute.
        assert!(
            out.disputed_answers.is_empty(),
            "corroborating proofs should not produce a dispute, got {:?}",
            out.disputed_answers
        );
    }

    #[test]
    fn dispute_detected_when_rulebooks_produce_different_bindings() {
        // Source fact: `subject(x)`. Two rulebooks contribute
        // contradictory classifications:
        //   rb-strict: classify(X, prohibited) :- subject(X).
        //   rb-lenient: classify(X, allowed)   :- subject(X).
        // Query: ?- classify(x, Status).
        // Engine returns two proofs: one binds Status=prohibited
        // (from rb-strict's rule), one binds Status=allowed (from
        // rb-lenient's). Distinct rulebooks AND distinct bindings →
        // dispute.
        use logic_core::{atom, compound, var};
        let text = "x";
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: text.into(),
                ..pipeline_doc()
            },
            ir_document: make_ir(vec![
                fact_node("F1", compound("subject", vec![atom("x")]), 0, text.len()),
                query_node(
                    "Q1",
                    compound("classify", vec![atom("x"), Term::Var(var("Status"))]),
                ),
            ]),
        };
        let xv = var("X");
        let strict_rule = compound(
            "definitional",
            vec![
                compound("classify", vec![Term::Var(xv.clone()), atom("prohibited")]),
                logic_core::logic_list(vec![compound("subject", vec![Term::Var(xv.clone())])]),
            ],
        );
        let lenient_rule = compound(
            "definitional",
            vec![
                compound("classify", vec![Term::Var(xv.clone()), atom("allowed")]),
                logic_core::logic_list(vec![compound("subject", vec![Term::Var(xv.clone())])]),
            ],
        );
        let rb_strict = rulebook_ir("rb-strict", vec![rule_node("Rs", strict_rule)]);
        let rb_lenient = rulebook_ir("rb-lenient", vec![rule_node("Rl", lenient_rule)]);
        let out = run_with_rulebooks(
            input,
            AdjudicationId::new("adj-dispute"),
            make_clock(),
            None,
            &[
                (rb_strict, ClauseProvenance::new("rb-strict", TrustTier::Tentative)),
                (rb_lenient, ClauseProvenance::new("rb-lenient", TrustTier::Tentative)),
            ],
        );
        assert_eq!(
            out.disputed_answers.len(),
            1,
            "expected exactly one disputed answer, got {:?}",
            out.disputed_answers
        );
        let dispute = &out.disputed_answers[0];
        assert_eq!(
            dispute.resolution_required,
            ResolutionRequirement::HumanReview
        );
        // Two candidates, one from each rulebook.
        assert_eq!(dispute.candidates.len(), 2);
        let mut rulebook_ids: Vec<&str> = dispute
            .candidates
            .iter()
            .flat_map(|c| c.source_rulebooks.iter().map(|s| s.as_str()))
            .collect();
        rulebook_ids.sort();
        rulebook_ids.dedup();
        // The source document's fact contributes "doc1" provenance
        // (the default Authoritative source). Plus rb-strict and
        // rb-lenient for the rules.
        assert!(rulebook_ids.contains(&"rb-strict"));
        assert!(rulebook_ids.contains(&"rb-lenient"));
    }

    #[test]
    fn no_dispute_from_corroborating_pair_even_with_within_rulebook_ambiguity() {
        // Synthesize a 3-proof DAG by hand and feed it directly to
        // detect_disputes. The scenario:
        //   proof_a: bindings X, from {rb-alpha}
        //   proof_b: bindings X, from {rb-beta}  (corroborates a)
        //   proof_c: bindings Y, from {rb-alpha} (within-rb ambiguity)
        //
        // Pairwise:
        //   (a, b): different rulebooks, SAME bindings → no
        //   (a, c): SAME rulebooks ({rb-alpha}), different bindings → no
        //   (b, c): different rulebooks AND different bindings → DISPUTE
        //
        // So the joint per-pair check correctly identifies a
        // genuine cross-rulebook disagreement: rb-beta says X, but
        // rb-alpha (via its Y proof) says Y. The (a, b) corroboration
        // doesn't undo that. This is the *correct* behaviour — the
        // engine cannot tell which of rb-alpha's two answers it
        // intends, but the framework should still surface that
        // rb-beta and rb-alpha disagree on at least one reading.
        use logic_core::{atom, var, compound, Substitution};
        use logic_engine::{FactId, RuleId};
        let mk_bindings = |v_name: &str, t: logic_core::Term| -> Substitution {
            Substitution::empty().extend(var(v_name).id, t)
        };
        let proof_a = logic_engine::Proof {
            bindings: mk_bindings("Status", atom("x")),
            steps: vec![],
            via_facts: vec![FactId(100)],
            via_rules: vec![RuleId(200)],
        };
        let proof_b = logic_engine::Proof {
            bindings: mk_bindings("Status", atom("x")),
            steps: vec![],
            via_facts: vec![FactId(101)],
            via_rules: vec![RuleId(201)],
        };
        let proof_c = logic_engine::Proof {
            bindings: mk_bindings("Status", atom("y")),
            steps: vec![],
            via_facts: vec![FactId(102)],
            via_rules: vec![RuleId(202)],
        };
        let dag = logic_engine::ProofDAG {
            root_query: compound("classify", vec![atom("z"), Term::Var(var("Status"))]),
            proofs: vec![proof_a, proof_b, proof_c],
        };
        let answer = AdjudicationResult {
            query: dag.root_query.clone(),
            result: logic_engine::SearchResult::EnumerateAllResult {
                dag,
                probability: 1.0,
            },
        };
        let mut table = ClauseProvenanceTable::default();
        // a and c → rb-alpha; b → rb-beta.
        table.fact_provenance.insert(
            FactId(100),
            ClauseProvenance::new("rb-alpha", TrustTier::Tentative),
        );
        table.rule_provenance.insert(
            RuleId(200),
            ClauseProvenance::new("rb-alpha", TrustTier::Tentative),
        );
        table.fact_provenance.insert(
            FactId(101),
            ClauseProvenance::new("rb-beta", TrustTier::Tentative),
        );
        table.rule_provenance.insert(
            RuleId(201),
            ClauseProvenance::new("rb-beta", TrustTier::Tentative),
        );
        table.fact_provenance.insert(
            FactId(102),
            ClauseProvenance::new("rb-alpha", TrustTier::Tentative),
        );
        table.rule_provenance.insert(
            RuleId(202),
            ClauseProvenance::new("rb-alpha", TrustTier::Tentative),
        );
        let disputes = detect_disputes(&[answer], &table);
        // Joint per-pair check: (b, c) qualifies as a dispute pair.
        // The detector flags the answer; the candidate list shows
        // all three proofs so a reviewer can see the full picture.
        assert_eq!(disputes.len(), 1, "expected one disputed answer");
        assert_eq!(disputes[0].candidates.len(), 3);

    }

    #[test]
    fn detect_disputes_with_empty_attribution_returns_empty() {
        // Sanity: feeding detect_disputes an empty answer list (or
        // answers with no EnumerateAll results) returns an empty vec.
        let answers: Vec<AdjudicationResult> = Vec::new();
        let table = ClauseProvenanceTable::default();
        assert!(detect_disputes(&answers, &table).is_empty());
    }

    #[test]
    fn run_with_rulebooks_uses_enumerate_all_when_rulebooks_attached() {
        // Sanity: with rulebooks the engine runs in EnumerateAll
        // (so dispute detection can see all proofs). With no
        // rulebooks the audit trail records AutoDetect.
        use logic_core::atom;
        let text = "x";
        let f = fact_node("F1", atom("ok"), 0, text.len());
        let q = query_node("Q1", atom("ok"));
        let with_rb = PipelineInput {
            document: PipelineDocument {
                normalized_text: text.into(),
                ..pipeline_doc()
            },
            ir_document: make_ir(vec![f.clone(), q.clone()]),
        };
        let trivial_rb = rulebook_ir("rb-trivial", vec![]);
        let out_with = run_with_rulebooks(
            with_rb,
            AdjudicationId::new("adj-mode-rb"),
            make_clock(),
            None,
            &[(trivial_rb, ClauseProvenance::new("rb-trivial", TrustTier::Tentative))],
        );
        match &out_with.audit_trail.engine_artifacts.as_ref().unwrap().search_mode {
            SearchMode::EnumerateAll => {}
            other => panic!("expected EnumerateAll, got {:?}", other),
        }

        let without_rb = PipelineInput {
            document: PipelineDocument {
                normalized_text: text.into(),
                ..pipeline_doc()
            },
            ir_document: make_ir(vec![f, q]),
        };
        let out_without = run_with_rulebooks(
            without_rb,
            AdjudicationId::new("adj-mode-norb"),
            make_clock(),
            None,
            &[],
        );
        match &out_without.audit_trail.engine_artifacts.as_ref().unwrap().search_mode {
            SearchMode::AutoDetect => {}
            other => panic!("expected AutoDetect, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------
    // ADJ16 step 4 — agreement-weighted rulebook tests
    // -----------------------------------------------------------------

    fn def_rule_ir(doc_id: &str, head: logic_core::Term, body: Vec<logic_core::Term>) -> IRDocument {
        use logic_core::{compound, logic_list};
        let rule_term = compound("definitional", vec![head, logic_list(body)]);
        IRDocument {
            document_id: IRDocumentId::new(doc_id),
            nodes: vec![rule_node("R1", rule_term)],
            edges: vec![],
        }
    }

    /// Helper: extract the `(weight, head, body_list_term)` triple
    /// from a probabilistic rule term. Panics if shape is wrong.
    fn unpack_probabilistic(term: &logic_core::Term) -> (f64, &logic_core::Term, &logic_core::Term) {
        match term {
            logic_core::Term::Compound { functor, args } if functor == "probabilistic" && args.len() == 3 => {
                let weight = match &args[0] {
                    logic_core::Term::Num(logic_core::Number::Float(f)) => *f,
                    other => panic!("expected float weight, got {:?}", other),
                };
                (weight, &args[1], &args[2])
            }
            other => panic!("expected probabilistic compound, got {:?}", other),
        }
    }

    #[test]
    fn agreement_weight_empty_rulebooks_returns_empty_doc() {
        let merged = compute_agreement_weighted_rulebook(&[], "out");
        assert_eq!(merged.document_id.0, "out");
        assert!(merged.nodes.is_empty());
        assert!(merged.edges.is_empty());
    }

    #[test]
    fn agreement_weight_single_rulebook_assigns_weight_one() {
        use logic_core::{atom, compound};
        let rb = def_rule_ir(
            "rb1",
            compound("non_compliant", vec![atom("p")]),
            vec![compound("prohibited", vec![atom("matches")])],
        );
        let merged = compute_agreement_weighted_rulebook(&[&rb], "out");
        assert_eq!(merged.nodes.len(), 1);
        let (w, _, _) = unpack_probabilistic(&merged.nodes[0].term);
        assert!((w - 1.0).abs() < 1e-9, "weight should be 1.0 for single-rulebook case");
    }

    #[test]
    fn agreement_weight_two_rulebooks_full_agreement_yields_weight_one() {
        use logic_core::{atom, compound};
        let h = compound("non_compliant", vec![atom("p")]);
        let b = vec![compound("prohibited", vec![atom("matches")])];
        let rb1 = def_rule_ir("rb1", h.clone(), b.clone());
        let rb2 = def_rule_ir("rb2", h.clone(), b.clone());
        let merged = compute_agreement_weighted_rulebook(&[&rb1, &rb2], "out");
        // Single dedup-merged rule with weight 2/2 = 1.0.
        assert_eq!(merged.nodes.len(), 1);
        let (w, _, _) = unpack_probabilistic(&merged.nodes[0].term);
        assert!((w - 1.0).abs() < 1e-9, "weight should be 1.0 when both rulebooks agree");
    }

    #[test]
    fn agreement_weight_two_rulebooks_partial_overlap_yields_proportional_weight() {
        // rb1: rule A, rule B.
        // rb2: rule A only.
        // After merge: rule A weight 2/2 = 1.0, rule B weight 1/2 = 0.5.
        use logic_core::{atom, compound, logic_list};
        let a_head = compound("non_compliant", vec![atom("p")]);
        let a_body = vec![compound("prohibited", vec![atom("matches")])];
        let b_head = compound("flagged", vec![atom("p")]);
        let b_body = vec![compound("declared", vec![atom("lighter")])];
        let rb1 = IRDocument {
            document_id: IRDocumentId::new("rb1"),
            nodes: vec![
                rule_node(
                    "R1",
                    compound("definitional", vec![a_head.clone(), logic_list(a_body.clone())]),
                ),
                rule_node(
                    "R2",
                    compound("definitional", vec![b_head.clone(), logic_list(b_body.clone())]),
                ),
            ],
            edges: vec![],
        };
        let rb2 = def_rule_ir("rb2", a_head.clone(), a_body.clone());
        let merged = compute_agreement_weighted_rulebook(&[&rb1, &rb2], "out");
        assert_eq!(merged.nodes.len(), 2);
        // Find by head term identity (insertion order is rule A
        // first because it appears first in rb1).
        let (w1, h1, _) = unpack_probabilistic(&merged.nodes[0].term);
        let (w2, h2, _) = unpack_probabilistic(&merged.nodes[1].term);
        assert_eq!(h1, &a_head);
        assert!((w1 - 1.0).abs() < 1e-9, "rule A weight should be 1.0, got {}", w1);
        assert_eq!(h2, &b_head);
        assert!((w2 - 0.5).abs() < 1e-9, "rule B weight should be 0.5, got {}", w2);
    }

    #[test]
    fn agreement_weight_three_rulebooks_no_overlap_yields_one_third_each() {
        use logic_core::{atom, compound};
        let rb1 = def_rule_ir(
            "rb1",
            compound("rule_a", vec![atom("p")]),
            vec![atom("x")],
        );
        let rb2 = def_rule_ir(
            "rb2",
            compound("rule_b", vec![atom("p")]),
            vec![atom("y")],
        );
        let rb3 = def_rule_ir(
            "rb3",
            compound("rule_c", vec![atom("p")]),
            vec![atom("z")],
        );
        let merged = compute_agreement_weighted_rulebook(&[&rb1, &rb2, &rb3], "out");
        assert_eq!(merged.nodes.len(), 3);
        for node in &merged.nodes {
            let (w, _, _) = unpack_probabilistic(&node.term);
            assert!(
                (w - (1.0 / 3.0)).abs() < 1e-9,
                "each rule should have weight 1/3, got {}",
                w
            );
        }
    }

    #[test]
    fn agreement_weight_dedups_within_a_single_rulebook() {
        // A single rulebook listing the same rule twice shouldn't
        // inflate the count to 2/1 = 2.0. The dedup-within-rulebook
        // logic enforces "this rulebook contributes at most 1 to
        // each rule's count".
        use logic_core::{atom, compound, logic_list};
        let head = compound("non_compliant", vec![atom("p")]);
        let body = vec![compound("prohibited", vec![atom("matches")])];
        let rule_term = compound("definitional", vec![head, logic_list(body)]);
        let rb = IRDocument {
            document_id: IRDocumentId::new("rb1"),
            nodes: vec![
                rule_node("R1", rule_term.clone()),
                rule_node("R2", rule_term),
            ],
            edges: vec![],
        };
        let merged = compute_agreement_weighted_rulebook(&[&rb], "out");
        assert_eq!(merged.nodes.len(), 1);
        let (w, _, _) = unpack_probabilistic(&merged.nodes[0].term);
        assert!((w - 1.0).abs() < 1e-9, "weight should be 1.0 (1/1), not 2.0");
    }

    #[test]
    fn agreement_weight_passes_through_non_definitional_rules() {
        // A probabilistic, constraint, or default rule should pass
        // through unchanged. Currently the function preserves the
        // term and reuses it once.
        use logic_core::{atom, compound, logic_list, float};
        let prob_term = compound(
            "probabilistic",
            vec![float(0.7), atom("h"), logic_list(vec![atom("b")])],
        );
        let constraint_term = compound("constraint", vec![logic_list(vec![atom("c")])]);
        let rb = IRDocument {
            document_id: IRDocumentId::new("rb"),
            nodes: vec![
                rule_node("R1", prob_term.clone()),
                rule_node("R2", constraint_term.clone()),
            ],
            edges: vec![],
        };
        let merged = compute_agreement_weighted_rulebook(&[&rb], "out");
        assert_eq!(merged.nodes.len(), 2);
        // First should be the probabilistic (preserved as-is), then
        // the constraint.
        assert_eq!(merged.nodes[0].term, prob_term);
        assert_eq!(merged.nodes[1].term, constraint_term);
    }

    #[test]
    fn agreement_weighted_rulebook_feeds_run_with_rulebooks() {
        // End-to-end smoke: the function's output can be fed back
        // into run_with_rulebooks. Build two rulebooks that agree on
        // a bridging rule, merge them, run the pipeline against the
        // merged rulebook on a tiny source IR.
        use logic_core::{atom, compound, logic_list};
        let text = "x";
        let f = fact_node("F1", atom("a"), 0, text.len());
        let q = query_node("Q1", atom("b"));
        let input = PipelineInput {
            document: PipelineDocument {
                normalized_text: text.into(),
                ..pipeline_doc()
            },
            ir_document: make_ir(vec![f, q]),
        };
        // Both rulebooks supply the same bridging rule b :- a.
        let rule_term = compound("definitional", vec![atom("b"), logic_list(vec![atom("a")])]);
        let rb1 = IRDocument {
            document_id: IRDocumentId::new("rb1"),
            nodes: vec![rule_node("R1", rule_term.clone())],
            edges: vec![],
        };
        let rb2 = IRDocument {
            document_id: IRDocumentId::new("rb2"),
            nodes: vec![rule_node("R1", rule_term)],
            edges: vec![],
        };
        let merged = compute_agreement_weighted_rulebook(&[&rb1, &rb2], "rb-merged-v1");
        // Weight is 1.0 (full agreement) → probabilistic rule.
        let out = run_with_rulebooks(
            input,
            AdjudicationId::new("adj-step4-end-to-end"),
            make_clock(),
            None,
            &[(
                merged,
                ClauseProvenance::new("rb-merged-v1", TrustTier::Reviewed),
            )],
        );
        // Engine should derive `b` (probability 1.0 means certain).
        match &out.verdict {
            Verdict::Resolved { answers } => assert_eq!(answers.len(), 1),
            other => panic!("expected Resolved, got {other:?}"),
        }
        // Provenance attributes the rule to the merged rulebook id.
        let table = out.clause_provenance.as_ref().expect("provenance");
        assert_eq!(table.rule_provenance.len(), 1);
        let prov = table.rule_provenance.values().next().unwrap();
        assert_eq!(prov.source_rulebook_id, "rb-merged-v1");
    }
}
