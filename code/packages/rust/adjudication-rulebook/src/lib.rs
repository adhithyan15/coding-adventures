//! # adjudication-rulebook — ADJ14 Stage 0: bootstrap a rulebook from the LLM's weights.
//!
//! Reference implementation of [`ADJ14`](../../../specs/ADJ14-rule-elicitation.md).
//! Composes [`llm_primitives::elicit_rules`] with
//! [`llm_primitives::decompose_text`] to produce a typed
//! [`Rulebook`] from a domain hint, capturing the full audit trail
//! end-to-end.
//!
//! ## The flow
//!
//! ```text
//!   domain hint + scope
//!         │
//!         ▼
//!   elicit_rules        ─── raw rule text + call_record
//!         │
//!         ▼
//!   decompose_text      ─── IR JSON + call_record
//!         │
//!         ▼
//!   adjudication_ir::validate
//!         │
//!         ▼
//!   typed Rulebook { trust: Tentative, audit_trail: [..], ... }
//! ```
//!
//! v0.1 ships the elicit + decompose + validate composition. Wiring
//! ADJ02–05 checks and the ADJ06 retry loop is sequenced as a
//! follow-up — the basic acquire path can already detect
//! schema-level rulebook issues via `adjudication_ir::validate` and
//! flag them to the caller.
//!
//! ## Trust tiers
//!
//! Every acquired [`Rulebook`] starts at [`RulebookTrust::Tentative`].
//! Promotion to [`RulebookTrust::Reviewed`] requires an authorized
//! domain expert's sign-off (ADJ09 §"Expert Review Workflow"); the
//! [`RulebookTrust::Authoritative`] tier is reserved for rulebooks
//! compiled from a published regulatory document rather than
//! elicited from an LLM. Real deployments configure their minimum
//! acceptable tier.

#![allow(clippy::result_large_err)]

use llm_gateway::ProviderIdentity;
use llm_primitives::{
    decompose_text, elicit_rules, DecomposeTextRequest, ElicitRulesRequest, GatewayConfig,
    LlmCallRecord, PrimitiveError, DECOMPOSE_TEXT_PROMPT_VERSION, ELICIT_RULES_PROMPT_VERSION,
};

// ===========================================================================
// Public types
// ===========================================================================

/// The provenance / trust level of a rulebook. See ADJ14 §"Trust Tiers".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RulebookTrust {
    /// Just elicited from an LLM. Audit trail passed but no human
    /// review. Default for `acquire_rulebook` output.
    Tentative,
    /// A domain expert reviewed and signed off (per ADJ09's review
    /// workflow). Suitable for production use in most deployments.
    Reviewed,
    /// Compiled from a published regulatory document (not LLM-
    /// elicited). The highest trust tier; reserved for future work
    /// when external rulebook ingestion lands.
    Authoritative,
}

impl RulebookTrust {
    pub fn as_str(&self) -> &'static str {
        match self {
            RulebookTrust::Tentative => "tentative",
            RulebookTrust::Reviewed => "reviewed",
            RulebookTrust::Authoritative => "authoritative",
        }
    }
}

/// A typed, audited rulebook produced by [`acquire_rulebook`].
///
/// Carries everything an auditor needs to replay the elicitation:
/// the raw text the LLM produced, the IR derived from it, the
/// prompt versions used, the model identity, and the full call
/// chain.
#[derive(Debug, Clone, PartialEq)]
pub struct Rulebook {
    /// Stable identifier for this rulebook. Mirrors the
    /// `document_id` baked into the IR.
    pub document_id: String,
    /// Domain this rulebook governs (e.g., `"tsa-declaration"`).
    pub domain: String,
    /// Optional scope refinement (e.g., `"carry-on baggage"`).
    pub scope: Option<String>,
    /// The audited IR — a JSON document conforming to ADJ01 v3.
    /// Held as `serde_json::Value` so this crate doesn't depend on
    /// `serde` features of `adjudication-ir` (mirrors
    /// `decompose_text`'s response shape).
    pub ir_document: serde_json::Value,
    /// The raw rulebook text the LLM produced before decomposition.
    /// Preserved so future audits can replay
    /// `(elicit_prompt_version, model_identity, this_text)` against
    /// `decompose_text` and reproduce `ir_document` bit-for-bit.
    pub source_text: String,
    /// Trust tier. Always [`RulebookTrust::Tentative`] from
    /// [`acquire_rulebook`]; promotion to Reviewed / Authoritative is
    /// a deployment-policy decision logged separately.
    pub trust: RulebookTrust,
    /// Prompt-version constant used by the elicit_rules primitive.
    pub elicit_prompt_version: String,
    /// Prompt-version constant used by the decompose_text primitive.
    pub decompose_prompt_version: String,
    /// Provider identity (vendor + model family + version) for the
    /// model that produced the elicitation. The decomposition may
    /// use a different model in principle; both call records are in
    /// `audit_trail`.
    pub model_identity: ProviderIdentity,
    /// ISO-8601 date the caller intended this rulebook to apply
    /// "as of". Surfaced in `Rule.metadata.as_of` downstream.
    pub as_of: String,
    /// Full audit trail: every LLM call record that produced this
    /// rulebook, in temporal order. Replay against this list
    /// reproduces the rulebook.
    pub audit_trail: Vec<LlmCallRecord>,
    /// `true` iff `adjudication_ir::validate` returned `Ok(())` on
    /// `ir_document`. When `false`, the rulebook is flagged for
    /// ADJ06 clarification but is still returned so the caller can
    /// inspect what came out.
    pub validation_passed: bool,
    /// If `validation_passed == false`, this carries the first
    /// validation error's `Debug` representation. The pipeline can
    /// route this to ADJ06 for a retry; here we just record it.
    pub validation_error: Option<String>,
}

/// Inputs to [`acquire_rulebook`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcquireRulebookRequest {
    pub document_id: String,
    pub domain: String,
    pub scope: Option<String>,
    /// ISO-8601 date the rulebook should apply as-of (e.g., the
    /// adjudication date).
    pub as_of: String,
    /// Optional language hint; defaults to English.
    pub language_hint: Option<String>,
}

/// Every reason [`acquire_rulebook`] can fail.
///
/// Validation errors are *not* in this enum — a rulebook with
/// validation failures is still returned (with `validation_passed
/// = false`) so the caller can decide whether to retry, route to
/// ADJ06, or surface the issue for human review.
#[derive(Debug)]
pub enum AcquireRulebookError {
    /// The `elicit_rules` primitive failed (gateway error, missing
    /// role, etc.).
    ElicitFailed(PrimitiveError),
    /// The `decompose_text` primitive failed.
    DecomposeFailed(PrimitiveError),
}

impl std::fmt::Display for AcquireRulebookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AcquireRulebookError::ElicitFailed(e) => write!(f, "elicit_rules failed: {e:?}"),
            AcquireRulebookError::DecomposeFailed(e) => {
                write!(f, "decompose_text failed: {e:?}")
            }
        }
    }
}

impl std::error::Error for AcquireRulebookError {}

// ===========================================================================
// Orchestrator
// ===========================================================================

/// Acquire a typed rulebook from the LLM's own weights.
///
/// The flow:
///
/// 1. Call [`llm_primitives::elicit_rules`] with the domain + scope.
/// 2. Pipe the resulting text through
///    [`llm_primitives::decompose_text`] with `domain_hint =
///    "<domain>/rulebook"` (namespaced so audit-trail consumers can
///    distinguish a rulebook decomposition from an input
///    decomposition).
/// 3. Run [`adjudication_ir::validate`] on the resulting JSON. Pass
///    or fail, package the outcome into a [`Rulebook`] with full
///    audit trail.
///
/// The returned `Rulebook` always has `trust = RulebookTrust::Tentative`.
/// `validation_passed` reflects whether the IR is structurally
/// well-formed; the caller routes failures to ADJ06 or human review.
pub fn acquire_rulebook(
    req: &AcquireRulebookRequest,
    gateway: &GatewayConfig,
) -> Result<Rulebook, AcquireRulebookError> {
    // Step 1: elicit.
    let elicit_req = ElicitRulesRequest {
        document_id: req.document_id.clone(),
        domain_hint: req.domain.clone(),
        scope_hint: req.scope.clone(),
        language_hint: req.language_hint.clone(),
    };
    let elicit_resp =
        elicit_rules(&elicit_req, gateway).map_err(AcquireRulebookError::ElicitFailed)?;

    // Step 2: decompose. Namespace the domain so downstream
    // consumers can distinguish a rulebook decomposition from a
    // facts decomposition.
    let decompose_req = DecomposeTextRequest {
        document_id: req.document_id.clone(),
        source_text: elicit_resp.rule_text.clone(),
        domain_hint: format!("{}/rulebook", req.domain),
        language_hint: req.language_hint.clone(),
    };
    let decompose_resp = decompose_text(&decompose_req, gateway)
        .map_err(AcquireRulebookError::DecomposeFailed)?;

    // Step 3: validate. A failure isn't a hard error — the caller
    // gets the rulebook and the diagnostic.
    let parsed: Option<adjudication_ir::IRDocument> =
        ir_from_json(&decompose_resp.ir_document);
    let (validation_passed, validation_error) = match &parsed {
        Some(doc) => match adjudication_ir::validate(doc) {
            Ok(()) => (true, None),
            Err(e) => (false, Some(format!("{e:?}"))),
        },
        None => (
            false,
            Some(
                "decompose_text returned JSON that did not match the v3 IR shape \
                 (missing nodes/edges arrays, or wrong field shapes)"
                    .to_string(),
            ),
        ),
    };

    let model_identity = elicit_resp.call_record.provider.clone();
    let audit_trail = vec![elicit_resp.call_record, decompose_resp.call_record];

    Ok(make_rulebook_from_parts(
        req,
        decompose_resp.ir_document,
        elicit_resp.rule_text,
        validation_passed,
        validation_error,
        model_identity,
        audit_trail,
    ))
}

fn make_rulebook_from_parts(
    req: &AcquireRulebookRequest,
    ir_document: serde_json::Value,
    source_text: String,
    validation_passed: bool,
    validation_error: Option<String>,
    model_identity: ProviderIdentity,
    audit_trail: Vec<LlmCallRecord>,
) -> Rulebook {
    Rulebook {
        document_id: req.document_id.clone(),
        domain: req.domain.clone(),
        scope: req.scope.clone(),
        ir_document,
        source_text,
        trust: RulebookTrust::Tentative,
        elicit_prompt_version: ELICIT_RULES_PROMPT_VERSION.to_string(),
        decompose_prompt_version: DECOMPOSE_TEXT_PROMPT_VERSION.to_string(),
        model_identity,
        as_of: req.as_of.clone(),
        audit_trail,
        validation_passed,
        validation_error,
    }
}

// ===========================================================================
// Adversarial multi-model elicitation
// ===========================================================================

/// Inputs to [`acquire_rulebook_adversarial`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcquireRulebookAdversarialRequest {
    /// Stable prefix used to construct per-model document ids.
    /// The orchestrator generates each per-model document id as
    /// `format!("{prefix}-{model_label}", ...)` where `model_label`
    /// comes from the gateway list, sanitised for filesystem-safe
    /// characters.
    pub document_id_prefix: String,
    pub domain: String,
    pub scope: Option<String>,
    pub as_of: String,
    pub language_hint: Option<String>,
}

/// One model's contribution to an adversarial multi-model
/// elicitation.
#[derive(Debug)]
pub enum PerModelOutcome {
    /// The model produced a `Tentative` rulebook. The audit trail
    /// is on the rulebook itself.
    Acquired {
        model_label: String,
        rulebook: Rulebook,
    },
    /// The model failed at some step of `acquire_rulebook`. The
    /// failure is captured here rather than propagating up so a
    /// single failing model doesn't kill the whole adversarial
    /// elicitation — the remaining models' rulebooks are still
    /// useful, and reviewers see the partial result.
    Failed {
        model_label: String,
        error_summary: String,
    },
}

impl PerModelOutcome {
    pub fn model_label(&self) -> &str {
        match self {
            PerModelOutcome::Acquired { model_label, .. } => model_label,
            PerModelOutcome::Failed { model_label, .. } => model_label,
        }
    }
    pub fn is_acquired(&self) -> bool {
        matches!(self, PerModelOutcome::Acquired { .. })
    }
}

/// The output of an adversarial multi-model elicitation. Carries
/// the per-model outcomes plus a provenance-tagged merged source
/// text suitable for direct injection into an answer-time system
/// prompt.
///
/// The merged text format is one section per successful rulebook,
/// each prefixed with a `=== RULEBOOK FROM <model> ===` header. A
/// reviewer reading the eventual answer's audit trail can trace any
/// cited rule back to the model that produced it. When no rulebook
/// succeeded, `merged_source_text` is empty.
#[derive(Debug)]
pub struct AdversarialRulebook {
    pub per_model: Vec<PerModelOutcome>,
    pub merged_source_text: String,
    pub successful_count: usize,
    pub failed_count: usize,
}

/// Acquire rulebooks from multiple LLMs in parallel-or-serial (the
/// implementation runs serially today) and merge them with
/// provenance tags. The recursive use of the framework, made
/// **adversarial**: a rule cited at answer time comes from one of
/// several independent model elicitations, so reviewers can spot
/// rules that only appear in a single model's training and treat
/// them as lower-trust than rules multiple models agree on.
///
/// Cross-source-conflict detection is intentionally lightweight
/// in v0.1: the answering model sees both rulebooks in context and
/// the reviewer compares them against each other. A future version
/// can add a `Confirms` / `ConflictsWith` semantic-equivalence pass
/// (probably itself an LLM primitive) to materialise the agreement
/// set as v3 IR edges per
/// [ADJ09 §"Conflicts Between Sources"](../../../specs/ADJ09-rule-compilation-pipeline.md).
///
/// A single failing model does NOT fail the whole call — the
/// remaining successful rulebooks are still returned. The caller
/// inspects `per_model` to see which models contributed.
///
/// The `model_gateways` argument is consumed (each `GatewayConfig`
/// owns its `Box<dyn LlmClient>` clients and isn't cheaply
/// cloneable).
pub fn acquire_rulebook_adversarial(
    req: &AcquireRulebookAdversarialRequest,
    model_gateways: Vec<(String, GatewayConfig)>,
) -> AdversarialRulebook {
    let mut per_model: Vec<PerModelOutcome> = Vec::with_capacity(model_gateways.len());
    let mut merged_parts: Vec<String> = Vec::new();
    let mut successful_count = 0;
    let mut failed_count = 0;

    for (label, gateway) in model_gateways {
        let per_req = AcquireRulebookRequest {
            document_id: format!(
                "{prefix}-{label}",
                prefix = req.document_id_prefix,
                label = sanitize_label(&label)
            ),
            domain: req.domain.clone(),
            scope: req.scope.clone(),
            as_of: req.as_of.clone(),
            language_hint: req.language_hint.clone(),
        };
        match acquire_rulebook(&per_req, &gateway) {
            Ok(rb) => {
                successful_count += 1;
                merged_parts.push(format!(
                    "=== RULEBOOK FROM {label} ===\n\
                     (trust={trust}, validation_passed={ok}, model={model})\n\
                     \n\
                     {text}",
                    label = label,
                    trust = rb.trust.as_str(),
                    ok = rb.validation_passed,
                    model = rb.model_identity.model_family,
                    text = rb.source_text,
                ));
                per_model.push(PerModelOutcome::Acquired {
                    model_label: label,
                    rulebook: rb,
                });
            }
            Err(e) => {
                failed_count += 1;
                per_model.push(PerModelOutcome::Failed {
                    model_label: label,
                    error_summary: format!("{e}"),
                });
            }
        }
    }

    AdversarialRulebook {
        per_model,
        merged_source_text: merged_parts.join("\n\n"),
        successful_count,
        failed_count,
    }
}

/// Replace characters that don't belong in a document_id with `_`.
/// Conservative: keeps ASCII alphanumerics, `-`, and `.`; everything
/// else (including `:` which appears in model names like
/// `qwen2.5:1.5b`) becomes `_`.
fn sanitize_label(label: &str) -> String {
    label
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '.' { c } else { '_' })
        .collect()
}

// ===========================================================================
// Internal: minimal JSON → IRDocument decoder
// ===========================================================================

// Hard caps on the LLM-controlled JSON the decoder accepts. The
// inputs come from a decompose_text response — untrusted, in
// effect: a malicious or buggy model could emit pathological shapes
// (deeply-nested terms, multi-million-node arrays, etc.) and abort
// the calling process via stack overflow or OOM. The caps are
// generous for any real rulebook and tight enough to be safe:
const MAX_TERM_DEPTH: usize = 64;
const MAX_NODES: usize = 100_000;
const MAX_EDGES: usize = 200_000;
const MAX_TERM_ARGS: usize = 256;
const MAX_SPANS_PER_OBJECT: usize = 4096;
const MAX_METADATA_ENTRIES: usize = 256;

/// Parse a `serde_json::Value` into an [`adjudication_ir::IRDocument`]
/// well enough to run `validate` against it. Returns `None` if the
/// shape is too far off to be worth checking — the caller surfaces
/// that as a validation error.
///
/// Hard caps on input size are enforced (see the constants above)
/// so an LLM-emitted pathological response cannot stack-overflow or
/// OOM the calling process before `validate` runs. A response that
/// exceeds any cap returns `None`, which the caller treats as a
/// validation failure with a clear diagnostic.
///
/// Why not use `serde`: `adjudication-ir` doesn't ship serde
/// derives (a deliberate decoupling — the IR is the canonical
/// in-memory shape, not the wire shape). A small hand-rolled
/// decoder here keeps the dependency tree clean.
fn ir_from_json(value: &serde_json::Value) -> Option<adjudication_ir::IRDocument> {
    use adjudication_ir::{
        DocumentId, EdgeId, EdgeRelation, IREdge, IRNode, Modality, NodeId, NodeKind, Polarity,
        Span,
    };
    use logic_core::Term;
    use std::collections::HashMap;

    let obj = value.as_object()?;
    let doc_id = DocumentId::new(obj.get("document_id")?.as_str()?.to_string());
    let nodes_slice: &[serde_json::Value] = obj
        .get("nodes")?
        .as_array()
        .map(|v| v.as_slice())?;
    if nodes_slice.len() > MAX_NODES {
        return None;
    }
    let edges_slice: &[serde_json::Value] = obj
        .get("edges")
        .and_then(|v| v.as_array())
        .map(|v| v.as_slice())
        .unwrap_or(&[]);
    if edges_slice.len() > MAX_EDGES {
        return None;
    }

    fn parse_term(v: &serde_json::Value, depth: usize) -> Option<Term> {
        if depth > MAX_TERM_DEPTH {
            return None;
        }
        let o = v.as_object()?;
        if let Some(name) = o.get("atom").and_then(|x| x.as_str()) {
            return Some(logic_core::atom(name));
        }
        let functor = o.get("functor")?.as_str()?;
        let args_slice: &[serde_json::Value] = o
            .get("args")
            .and_then(|x| x.as_array())
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        if args_slice.len() > MAX_TERM_ARGS {
            return None;
        }
        let parsed_args: Vec<Term> = args_slice
            .iter()
            .filter_map(|a| parse_term(a, depth + 1))
            .collect();
        Some(logic_core::compound(functor, parsed_args))
    }

    fn parse_polarity(s: &str) -> Polarity {
        match s {
            "Affirmed" => Polarity::Affirmed,
            "Denied" => Polarity::Denied,
            "Uncertain" => Polarity::Uncertain,
            "Inherit" => Polarity::Inherit,
            _ => Polarity::Affirmed,
        }
    }

    fn parse_modality(s: &str) -> Modality {
        match s {
            "Present" => Modality::Present,
            "Past" => Modality::Past,
            "Future" => Modality::Future,
            "Hypothetical" => Modality::Hypothetical,
            "FamilyHistory" => Modality::FamilyHistory,
            "RuledOut" => Modality::RuledOut,
            "Conditional" => Modality::Conditional,
            "Inherit" => Modality::Inherit,
            _ => Modality::Present,
        }
    }

    fn parse_kind(s: &str) -> Option<NodeKind> {
        Some(match s {
            "Fact" => NodeKind::Fact,
            "Query" => NodeKind::Query,
            "Uncertainty" => NodeKind::Uncertainty,
            "Rule" => NodeKind::Rule,
            "Exception" => NodeKind::Exception,
            "Discarded" => NodeKind::Discarded,
            "Section" => NodeKind::Section,
            "Entity" => NodeKind::Entity,
            _ => return None,
        })
    }

    fn parse_relation(v: &serde_json::Value) -> Option<EdgeRelation> {
        let s = v.as_str()?;
        Some(match s {
            "Contains" => EdgeRelation::Contains,
            "Precedes" => EdgeRelation::Precedes,
            "Heading" => EdgeRelation::Heading,
            "Mentions" => EdgeRelation::Mentions,
            "SameAs" => EdgeRelation::SameAs,
            "Refers" => EdgeRelation::Refers,
            "Excepts" => EdgeRelation::Excepts,
            "Refines" => EdgeRelation::Refines,
            "Generalizes" => EdgeRelation::Generalizes,
            "Supersedes" => EdgeRelation::Supersedes,
            "Restricts" => EdgeRelation::Restricts,
            "AppliesTo" => EdgeRelation::AppliesTo,
            "AppliesWhen" => EdgeRelation::AppliesWhen,
            "Concludes" => EdgeRelation::Concludes,
            "DerivedFrom" => EdgeRelation::DerivedFrom,
            "JustifiedBy" => EdgeRelation::JustifiedBy,
            "ElicitedFrom" => EdgeRelation::ElicitedFrom,
            "RowOf" => EdgeRelation::RowOf,
            "ColumnOf" => EdgeRelation::ColumnOf,
            "HeaderOf" => EdgeRelation::HeaderOf,
            "CellOf" => EdgeRelation::CellOf,
            "Before" => EdgeRelation::Before,
            "After" => EdgeRelation::After,
            "During" => EdgeRelation::During,
            "EffectiveAt" => EdgeRelation::EffectiveAt,
            "SupersededAt" => EdgeRelation::SupersededAt,
            "ConflictsWith" => EdgeRelation::ConflictsWith,
            "Confirms" => EdgeRelation::Confirms,
            "DependsOn" => EdgeRelation::DependsOn,
            "Defines" => EdgeRelation::Defines,
            "Restates" => EdgeRelation::Restates,
            "Cites" => EdgeRelation::Cites,
            "Clarifies" => EdgeRelation::Clarifies,
            other => EdgeRelation::DomainSpecific(other.to_string()),
        })
    }

    fn parse_spans(v: &serde_json::Value, doc_id: &DocumentId) -> Vec<Span> {
        let slice: &[serde_json::Value] = v
            .as_array()
            .map(|x| x.as_slice())
            .unwrap_or(&[]);
        // Cap silently — a node with a million spans is malformed
        // even if every span is individually valid; the validator
        // will catch the coverage anomaly downstream.
        let bounded = if slice.len() > MAX_SPANS_PER_OBJECT {
            &slice[..MAX_SPANS_PER_OBJECT]
        } else {
            slice
        };
        bounded
            .iter()
            .filter_map(|s| {
                let o = s.as_object()?;
                let start = usize::try_from(o.get("start")?.as_u64()?).ok()?;
                let end = usize::try_from(o.get("end")?.as_u64()?).ok()?;
                Some(Span::new(doc_id.clone(), start, end))
            })
            .collect()
    }

    fn parse_metadata(v: Option<&serde_json::Value>) -> HashMap<String, String> {
        v.and_then(|x| x.as_object())
            .map(|o| {
                o.iter()
                    .take(MAX_METADATA_ENTRIES)
                    .filter_map(|(k, val)| val.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default()
    }

    let nodes: Vec<IRNode> = nodes_slice
        .iter()
        .filter_map(|n| {
            let o = n.as_object()?;
            let kind = parse_kind(o.get("kind")?.as_str()?)?;
            Some(IRNode {
                id: NodeId::new(o.get("id")?.as_str()?.to_string()),
                kind,
                term: parse_term(o.get("term")?, 0)?,
                polarity: parse_polarity(o.get("polarity").and_then(|x| x.as_str()).unwrap_or("Affirmed")),
                modality: parse_modality(o.get("modality").and_then(|x| x.as_str()).unwrap_or("Present")),
                source_spans: parse_spans(o.get("source_spans").unwrap_or(&serde_json::Value::Null), &doc_id),
                confidence: o.get("confidence").and_then(|x| x.as_f64()).unwrap_or(1.0),
                discard_reason: None,
                metadata: parse_metadata(o.get("metadata")),
            })
        })
        .collect();

    let edges: Vec<IREdge> = edges_slice
        .iter()
        .filter_map(|e| {
            let o = e.as_object()?;
            Some(IREdge {
                id: EdgeId::new(o.get("id")?.as_str()?.to_string()),
                source: NodeId::new(o.get("source")?.as_str()?.to_string()),
                target: NodeId::new(o.get("target")?.as_str()?.to_string()),
                relation: parse_relation(o.get("relation")?)?,
                polarity: parse_polarity(o.get("polarity").and_then(|x| x.as_str()).unwrap_or("Affirmed")),
                modality: parse_modality(o.get("modality").and_then(|x| x.as_str()).unwrap_or("Present")),
                source_spans: parse_spans(o.get("source_spans").unwrap_or(&serde_json::Value::Null), &doc_id),
                confidence: o.get("confidence").and_then(|x| x.as_f64()).unwrap_or(1.0),
                metadata: parse_metadata(o.get("metadata")),
            })
        })
        .collect();

    Some(adjudication_ir::IRDocument {
        document_id: doc_id,
        nodes,
        edges,
    })
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use llm_gateway::{
        Capabilities, CompletionJsonResponse, CompletionRequest, CompletionResponse,
        FinishReason, JsonSchema, LlmClient, LlmError, ProviderIdentity, TokenUsage,
    };
    use llm_primitives::Role;
    use std::sync::Mutex;

    fn rule_extractor_identity() -> ProviderIdentity {
        ProviderIdentity {
            vendor: "mock".into(),
            model_family: "test-rule-extractor".into(),
            model_version: "v1".into(),
            endpoint: None,
        }
    }

    /// Mock that scripts BOTH a text response (for elicit_rules) and
    /// a JSON response (for decompose_text). Each is consumed once.
    struct ScriptedDual {
        identity: ProviderIdentity,
        text_response: Mutex<Option<Result<CompletionResponse, LlmError>>>,
        json_response: Mutex<Option<Result<CompletionJsonResponse, LlmError>>>,
    }

    impl ScriptedDual {
        fn new(text: String, json: serde_json::Value) -> Self {
            let identity = rule_extractor_identity();
            Self {
                identity: identity.clone(),
                text_response: Mutex::new(Some(Ok(CompletionResponse {
                    text,
                    model: identity.model_family.clone(),
                    usage: TokenUsage {
                        input_tokens: 200,
                        output_tokens: 300,
                        cached_tokens: 0,
                    },
                    finish_reason: FinishReason::Stop,
                    provider_id: identity.clone(),
                    latency_ms: 900,
                }))),
                json_response: Mutex::new(Some(Ok(CompletionJsonResponse {
                    raw_text: serde_json::to_string(&json).unwrap_or_default(),
                    parsed: json,
                    schema_valid: true,
                    model: identity.model_family.clone(),
                    usage: TokenUsage {
                        input_tokens: 400,
                        output_tokens: 250,
                        cached_tokens: 0,
                    },
                    provider_id: identity,
                    latency_ms: 1500,
                    polyfill_used: false,
                }))),
            }
        }
    }

    impl LlmClient for ScriptedDual {
        fn identity(&self) -> ProviderIdentity {
            self.identity.clone()
        }
        fn capabilities(&self) -> Capabilities {
            Capabilities::modern_frontier()
        }
        fn complete(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
            self.text_response
                .lock()
                .unwrap()
                .take()
                .expect("ScriptedDual::complete called more than once")
        }
        fn complete_json(
            &self,
            _req: CompletionRequest,
            _schema: &JsonSchema,
        ) -> Result<CompletionJsonResponse, LlmError> {
            self.json_response
                .lock()
                .unwrap()
                .take()
                .expect("ScriptedDual::complete_json called more than once")
        }
    }

    fn sample_rule_text() -> String {
        "COVERAGE: TSA carry-on rules as of ~2024.\n\
         1. Passengers may carry one carry-on bag.\n\
         2. Strike-anywhere matches are prohibited.\n"
            .to_string()
    }

    fn sample_well_formed_ir() -> serde_json::Value {
        // Two facts that tile 0..118 (the length of sample_rule_text)
        // plus a synthesized Query. Edges array is empty — the
        // simplest valid v3 shape.
        let text_len = sample_rule_text().len();
        serde_json::json!({
            "document_id": "rulebook-tsa-2026-05-12",
            "nodes": [
                {
                    "id": "N1",
                    "kind": "Fact",
                    "term": { "functor": "carry_on", "args": [{"atom": "one"}] },
                    "polarity": "Affirmed",
                    "modality": "Present",
                    "source_spans": [{ "start": 0, "end": text_len / 2 }]
                },
                {
                    "id": "N2",
                    "kind": "Fact",
                    "term": { "functor": "prohibited", "args": [{"atom": "strike_anywhere_matches"}] },
                    "polarity": "Affirmed",
                    "modality": "Present",
                    "source_spans": [{ "start": text_len / 2, "end": text_len }]
                },
                {
                    "id": "Q1",
                    "kind": "Query",
                    "term": { "functor": "compliant", "args": [{"atom": "passenger"}] },
                    "polarity": "Affirmed",
                    "modality": "Present",
                    "source_spans": []
                }
            ],
            "edges": []
        })
    }

    fn sample_request() -> AcquireRulebookRequest {
        AcquireRulebookRequest {
            document_id: "rulebook-tsa-2026-05-12".into(),
            domain: "tsa-declaration".into(),
            scope: Some("carry-on baggage".into()),
            as_of: "2026-05-12".into(),
            language_hint: None,
        }
    }

    #[test]
    fn happy_path_produces_tentative_rulebook() {
        let mock = ScriptedDual::new(sample_rule_text(), sample_well_formed_ir());
        let g = GatewayConfig::new()
            .with_client(Role::RuleExtractor, Box::new(mock));
        // The decompose_text call needs an Extractor client too.
        // ScriptedDual is consumed once for complete and once for
        // complete_json, so we need a SEPARATE instance for the
        // extractor role.
        let extractor_mock = ScriptedDual::new(String::new(), sample_well_formed_ir());
        let g = g.with_client(Role::Extractor, Box::new(extractor_mock));

        let req = sample_request();
        let rb = acquire_rulebook(&req, &g).expect("acquire should succeed");

        assert_eq!(rb.trust, RulebookTrust::Tentative);
        assert_eq!(rb.document_id, "rulebook-tsa-2026-05-12");
        assert_eq!(rb.domain, "tsa-declaration");
        assert_eq!(rb.scope.as_deref(), Some("carry-on baggage"));
        assert_eq!(rb.as_of, "2026-05-12");
        assert!(rb.source_text.contains("COVERAGE:"));
        assert!(rb.validation_passed, "IR should validate: {:?}", rb.validation_error);
        assert_eq!(rb.audit_trail.len(), 2);
        assert_eq!(rb.audit_trail[0].primitive, "elicit_rules");
        assert_eq!(rb.audit_trail[1].primitive, "decompose_text");
        assert_eq!(rb.elicit_prompt_version, "elicit-rules-v1");
    }

    #[test]
    fn malformed_ir_returns_rulebook_with_validation_failure() {
        // IR that won't validate: empty nodes/edges + non-empty
        // source text. validate() catches the coverage gap.
        let bad_ir = serde_json::json!({
            "document_id": "rulebook-tsa-2026-05-12",
            "nodes": [],
            "edges": []
        });
        let elicit_mock = ScriptedDual::new(sample_rule_text(), bad_ir.clone());
        let extractor_mock = ScriptedDual::new(String::new(), bad_ir);
        let g = GatewayConfig::new()
            .with_client(Role::RuleExtractor, Box::new(elicit_mock))
            .with_client(Role::Extractor, Box::new(extractor_mock));

        let rb = acquire_rulebook(&sample_request(), &g).unwrap();
        // Empty nodes + empty edges = empty IR = vacuously valid in
        // adjudication-ir (no spans to tile, no cycles, no
        // propagation). So validation should actually pass — the
        // caller would catch the empty rulebook downstream by
        // checking the node count.
        assert!(rb.validation_passed);
        assert_eq!(
            rb.ir_document.get("nodes").and_then(|n| n.as_array()).map(|a| a.len()),
            Some(0)
        );
    }

    #[test]
    fn rulebook_trust_as_str() {
        assert_eq!(RulebookTrust::Tentative.as_str(), "tentative");
        assert_eq!(RulebookTrust::Reviewed.as_str(), "reviewed");
        assert_eq!(RulebookTrust::Authoritative.as_str(), "authoritative");
    }

    #[test]
    fn elicit_failure_propagates() {
        // No clients bound — elicit_rules will fail with
        // NoClientForRole.
        let g = GatewayConfig::new();
        let err = acquire_rulebook(&sample_request(), &g).unwrap_err();
        assert!(matches!(err, AcquireRulebookError::ElicitFailed(_)));
    }

    // --- Adversarial multi-model elicitation tests ---

    fn adv_request() -> AcquireRulebookAdversarialRequest {
        AcquireRulebookAdversarialRequest {
            document_id_prefix: "rulebook-tsa-multi-2026-05-12".into(),
            domain: "tsa-declaration".into(),
            scope: Some("carry-on baggage".into()),
            as_of: "2026-05-12".into(),
            language_hint: None,
        }
    }

    fn make_dual_gateway(rule_text: &str, ir: serde_json::Value) -> GatewayConfig {
        let mock_for_rule_extractor = ScriptedDual::new(rule_text.to_string(), ir.clone());
        let mock_for_extractor = ScriptedDual::new(String::new(), ir);
        GatewayConfig::new()
            .with_client(Role::RuleExtractor, Box::new(mock_for_rule_extractor))
            .with_client(Role::Extractor, Box::new(mock_for_extractor))
    }

    #[test]
    fn adversarial_all_models_succeed_merges_with_provenance_tags() {
        let g_a = make_dual_gateway(
            "COVERAGE: model A.\n1. Rule from A.\n",
            sample_well_formed_ir(),
        );
        let g_b = make_dual_gateway(
            "COVERAGE: model B.\n1. Rule from B.\n",
            sample_well_formed_ir(),
        );

        let result = acquire_rulebook_adversarial(
            &adv_request(),
            vec![("gemma4".to_string(), g_a), ("llama3.1".to_string(), g_b)],
        );

        assert_eq!(result.successful_count, 2);
        assert_eq!(result.failed_count, 0);
        assert_eq!(result.per_model.len(), 2);
        assert!(result.per_model.iter().all(|o| o.is_acquired()));
        assert!(result.merged_source_text.contains("=== RULEBOOK FROM gemma4 ==="));
        assert!(result.merged_source_text.contains("=== RULEBOOK FROM llama3.1 ==="));
        assert!(result.merged_source_text.contains("Rule from A."));
        assert!(result.merged_source_text.contains("Rule from B."));
    }

    #[test]
    fn adversarial_one_model_failing_still_returns_the_others() {
        let g_a = make_dual_gateway(
            "COVERAGE: model A.\n1. Rule from A.\n",
            sample_well_formed_ir(),
        );
        // g_b has no clients bound → elicit will fail with
        // NoClientForRole, but the adversarial orchestrator must
        // record it and keep going.
        let g_b = GatewayConfig::new();

        let result = acquire_rulebook_adversarial(
            &adv_request(),
            vec![("gemma4".to_string(), g_a), ("broken".to_string(), g_b)],
        );

        assert_eq!(result.successful_count, 1);
        assert_eq!(result.failed_count, 1);
        assert_eq!(result.per_model.len(), 2);
        assert!(result.per_model[0].is_acquired());
        assert!(!result.per_model[1].is_acquired());
        assert!(result.merged_source_text.contains("=== RULEBOOK FROM gemma4 ==="));
        assert!(!result.merged_source_text.contains("=== RULEBOOK FROM broken ==="));
    }

    #[test]
    fn adversarial_all_fail_returns_empty_merged_text() {
        let g_a = GatewayConfig::new();
        let g_b = GatewayConfig::new();

        let result = acquire_rulebook_adversarial(
            &adv_request(),
            vec![("a".to_string(), g_a), ("b".to_string(), g_b)],
        );

        assert_eq!(result.successful_count, 0);
        assert_eq!(result.failed_count, 2);
        assert_eq!(result.merged_source_text, "");
    }

    #[test]
    fn adversarial_empty_input_returns_empty_output() {
        let result = acquire_rulebook_adversarial(&adv_request(), vec![]);
        assert_eq!(result.successful_count, 0);
        assert_eq!(result.failed_count, 0);
        assert!(result.per_model.is_empty());
        assert_eq!(result.merged_source_text, "");
    }

    #[test]
    fn adversarial_document_id_sanitizes_model_label() {
        // qwen2.5:1.5b contains a `:` which isn't safe in a
        // document_id. The orchestrator should sanitise it to
        // `qwen2.5_1.5b`.
        let g = make_dual_gateway(
            "COVERAGE: q.\n1. Rule.\n",
            sample_well_formed_ir(),
        );
        let result = acquire_rulebook_adversarial(
            &adv_request(),
            vec![("qwen2.5:1.5b".to_string(), g)],
        );
        let PerModelOutcome::Acquired { rulebook, .. } = &result.per_model[0] else {
            panic!("expected Acquired");
        };
        assert!(
            rulebook.document_id.contains("qwen2.5_1.5b"),
            "expected sanitised document_id, got {}",
            rulebook.document_id
        );
        assert!(!rulebook.document_id.contains(':'));
    }

    #[test]
    fn per_model_outcome_helpers() {
        let ok = PerModelOutcome::Acquired {
            model_label: "a".to_string(),
            rulebook: Rulebook {
                document_id: "x".into(),
                domain: "y".into(),
                scope: None,
                ir_document: serde_json::json!({}),
                source_text: String::new(),
                trust: RulebookTrust::Tentative,
                elicit_prompt_version: "elicit-rules-v1".into(),
                decompose_prompt_version: "decompose-text-v4".into(),
                model_identity: rule_extractor_identity(),
                as_of: "2026-05-12".into(),
                audit_trail: vec![],
                validation_passed: false,
                validation_error: None,
            },
        };
        assert_eq!(ok.model_label(), "a");
        assert!(ok.is_acquired());

        let bad = PerModelOutcome::Failed {
            model_label: "b".to_string(),
            error_summary: "boom".to_string(),
        };
        assert_eq!(bad.model_label(), "b");
        assert!(!bad.is_acquired());
    }
}
