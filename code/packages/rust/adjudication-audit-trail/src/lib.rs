//! # adjudication-audit-trail — the IR is the audit trail
//!
//! Reference implementation of [`ADJ07`](../../../specs/ADJ07-audit-trail-schema.md).
//!
//! The framework's audit story is "every byte of every verdict chains
//! back to the source bytes of the original document, as data, not
//! commentary." This crate is the **shape** of that chain: a tree of
//! plain Rust types annotated with `serde` so the whole thing
//! round-trips through JSON.
//!
//! This is intentionally **pure data types** — no I/O, no behaviour
//! beyond `Serialize` / `Deserialize`. Producers (the checker passes
//! in `adjudication-coverage`, `-polarity-modality`, `-round-trip`,
//! `-adversarial`, plus the engine and the dialogue) build up an
//! [`AuditTrail`] as the adjudication runs. The trail is then handed
//! to whatever persistence layer the deployment uses (inline
//! response, append-only log, content-addressed storage).
//!
//! ## Why opaque [`IrNodePayload`]
//!
//! ADJ01 v2 ([`adjudication_ir::IRNode`]) is the canonical IR shape,
//! but `adjudication-ir` does not yet derive `Serialize` /
//! `Deserialize`. To keep this PR small and let the schema land
//! independently, [`IrNode`] in v0.1 stores the node payload as a
//! `serde_json::Value` plus the node id. v0.2 will replace the
//! `serde_json::Value` with a typed `adjudication_ir::IRNode` once
//! that crate gets serde derives.
//!
//! Consumers can already serialize their own typed IR nodes into
//! `serde_json::Value` via `serde_json::to_value` (after they
//! gain `Serialize`) and drop them into the audit trail unchanged —
//! when the upgrade happens, only the type signature changes; the
//! on-wire shape stays the same.
//!
//! ## Schema versioning
//!
//! Every top-level structure carries a `schema_version` string. The
//! v0.1 schema is `"ADJ07-v1"`. Bumping the schema (additive vs.
//! breaking) is the audited way to evolve the trail format.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Top-level identifiers
// ---------------------------------------------------------------------------

/// Stable per-run identifier. Conventionally a UUIDv7, but the schema
/// only requires a non-empty string so deployments can choose the
/// id scheme that fits their existing observability.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AdjudicationId(pub String);

impl AdjudicationId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

/// Per-document identifier, scoped to one adjudication.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DocumentId(pub String);

impl DocumentId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

/// Per-node identifier from the IR (mirrors `adjudication_ir::NodeId`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(pub String);

impl NodeId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

/// Per-turn identifier within an adjudication's dialogue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TurnId(pub u64);

// ---------------------------------------------------------------------------
// Top-level structure
// ---------------------------------------------------------------------------

/// The full audit trail of one adjudication run. Persistence-layer
/// neutral: a deployment can serialize this whole struct to one
/// JSON file, stream it incrementally as an append-only log, or
/// shard substructures across a database.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditTrail {
    pub adjudication_id: AdjudicationId,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub outcome: AdjudicationOutcome,
    pub documents: Vec<Document>,
    pub ir_nodes: Vec<IrNode>,
    pub checker_results: Vec<CheckerResult>,
    pub dialogue: Vec<DialogueTurn>,
    pub engine_artifacts: Option<EngineArtifacts>,
    pub configuration: Configuration,
    pub schema_version: String,
}

impl AuditTrail {
    /// Schema version constant. Bumped on any breaking change. Additive
    /// fields (with `#[serde(default)]`) do not require a bump.
    pub const CURRENT_SCHEMA_VERSION: &'static str = "ADJ07-v1";

    /// Construct an empty in-progress trail. Producers add documents,
    /// IR nodes, checker results, etc. as the adjudication runs.
    pub fn new(adjudication_id: AdjudicationId, started_at: impl Into<String>) -> Self {
        Self {
            adjudication_id,
            started_at: started_at.into(),
            completed_at: None,
            outcome: AdjudicationOutcome::InProgress,
            documents: Vec::new(),
            ir_nodes: Vec::new(),
            checker_results: Vec::new(),
            dialogue: Vec::new(),
            engine_artifacts: None,
            configuration: Configuration::default(),
            schema_version: Self::CURRENT_SCHEMA_VERSION.to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Documents
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub id: DocumentId,
    pub name: String,
    pub received_at: String,
    pub normalized_text: String,
    pub normalization: NormalizationRecord,
    /// Raw original bytes if the deployment policy retains them.
    /// Stored base64-encoded in JSON; absent means "normalized text
    /// only retained."
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_base64: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub appended_turns: Vec<AppendInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormalizationRecord {
    pub pipeline: String,
    pub version: String,
    #[serde(default)]
    pub options: BTreeMap<String, serde_json::Value>,
}

/// Records the byte-offset range that a clarification turn appended
/// to the normalized text. Lets the IR's span offsets remain valid
/// after a clarification grows the document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppendInfo {
    pub turn_id: TurnId,
    pub start_offset: usize,
    pub end_offset: usize,
    pub appended_at: String,
}

// ---------------------------------------------------------------------------
// IR nodes (opaque at v0.1)
// ---------------------------------------------------------------------------

/// An IR node payload as it appears in the trail. v0.1 stores the
/// node opaquely as `serde_json::Value` to keep this crate
/// independent of the (currently non-Serialize) `adjudication-ir`
/// types. The `id` is duplicated at this level so the trail is
/// indexable without parsing the payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IrNode {
    pub id: NodeId,
    pub document_id: DocumentId,
    /// The full ADJ01 v2 node serialized as a JSON value. v0.2 will
    /// replace this with a typed `adjudication_ir::IRNode` once
    /// that crate gets serde derives.
    pub payload: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Checker results
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckerResult {
    pub pass_name: PassName,
    pub pass_version: String,
    pub started_at: String,
    pub completed_at: String,
    pub outcome: PassOutcome,
    #[serde(default)]
    pub violations: Vec<Violation>,
    #[serde(default)]
    pub telemetry: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PassName {
    Adj02Coverage,
    Adj03PolarityModality,
    Adj04RoundTrip,
    Adj05Adversarial,
    /// ADJ22 typed-quantity coverage — for every numerical literal
    /// in the source, an overlapping IR node must carry a
    /// `quantity(value, unit)` compound. See
    /// [ADJ22](../../../specs/ADJ22-typed-quantity-coverage.md).
    Adj22TypedQuantity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PassOutcome {
    Passed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Violation {
    pub node_id: NodeId,
    pub pass_name: PassName,
    pub kind: ClarificationKind,
    /// Pass-specific extra fields. Each pass's spec defines its own
    /// `detail` schema; this stays open-ended so the trail crate
    /// doesn't have to be updated for every new pass detail.
    #[serde(default)]
    pub detail: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triggered_dialogue_turn: Option<TurnId>,
    #[serde(default)]
    pub resolved: bool,
}

// ---------------------------------------------------------------------------
// Dialogue (per ADJ06)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClarificationKind {
    UncoveredSpan,
    AmbiguousPolarity,
    AmbiguousModality,
    RoundTripDrift,
    AdversarialReading,
    InheritChainUnresolved,
    /// A numerical literal in the source did not surface as a
    /// `quantity(value, unit)` compound in any IR node — emitted
    /// by ADJ22.
    MissingQuantity,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogueRung {
    /// Re-prompt the same LLM with a correction.
    Rung1ReprompT,
    /// Ask a different LLM acting as second opinion.
    Rung2SecondOpinion,
    /// Ask a human reviewer.
    Rung3Human,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogueResponseSource {
    Llm,
    Human,
    Cached,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogueResponse {
    pub source: DialogueResponseSource,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_version: Option<String>,
    /// The clarification-prompt template version that produced this
    /// turn, e.g. `"clarification-v1"`. Same purpose as
    /// `LlmCallRecord::prompt_version` — replay matches on
    /// (`prompt_version`, `prompt_hash`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_version: Option<String>,
    /// Content-addressed hash of the prompt the LLM saw for this
    /// turn. Same FNV-1a-rendered-hex format as
    /// `LlmCallRecord::prompt_hash`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_hash: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogueOutcome {
    Resolved,
    Escalated,
    Abandoned,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogueTurn {
    pub turn_id: TurnId,
    pub at: String,
    pub triggering_violation: Option<usize>, // index into checker_results[*].violations
    pub rung: DialogueRung,
    pub question_text: String,
    pub response: DialogueResponse,
    pub outcome: DialogueOutcome,
}

// ---------------------------------------------------------------------------
// Engine artifacts
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SearchMode {
    FindFirst,
    EnumerateAll,
    AutoDetect,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KbSummary {
    pub fact_count: usize,
    pub rule_count: usize,
    pub fact_ids: Vec<String>,
    pub rule_ids: Vec<String>,
    pub all_certain: bool,
}

/// d-DNNF / SDD / naive — kept opaque at v0.1 so the trail crate
/// doesn't pin a probabilistic-inference encoding before LP19's
/// formula representation stabilises.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BooleanFormula {
    pub encoding: String,
    pub payload: serde_json::Value,
    pub fact_vars: BTreeMap<String, u32>,
    pub rule_vars: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WmcResult {
    pub probability: f64,
    pub method: String,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngineArtifacts {
    pub engine_version: String,
    pub search_mode: SearchMode,
    pub kb_summary: KbSummary,
    /// LP19's proof DAG. Kept as JSON until logic-engine ships its
    /// own typed Serialize.
    pub proof_dag: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula: Option<BooleanFormula>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wmc_result: Option<WmcResult>,
    pub answer: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Records which component (model, tagger, prompt set, rendering
/// function) ran with which version. *Reproducibility requires this*
/// — running the same input through the same `Configuration` should
/// produce the same `AuditTrail` modulo recorded non-determinism
/// (temperature, sampling seed).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct VersionedComponent {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub config: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Configuration {
    pub tagger: VersionedComponent,
    pub trigger_taxonomy: VersionedComponent,
    pub extractor_model: VersionedComponent,
    pub renderer_model: VersionedComponent,
    pub nli_model: VersionedComponent,
    pub adversary_model: VersionedComponent,
    pub judge_model: VersionedComponent,
    pub rendering_function: VersionedComponent,
    #[serde(default)]
    pub coverage_strictness: String,
    #[serde(default)]
    pub polarity_modality_strictness: String,
    #[serde(default)]
    pub round_trip_strictness: String,
    #[serde(default)]
    pub adversary_sample_rate: f64,
    #[serde(default)]
    pub escalation_policy: String,
    #[serde(default)]
    pub schema_version: String,
}

// ---------------------------------------------------------------------------
// Outcome
// ---------------------------------------------------------------------------

/// What happened at the end of the adjudication. `InProgress` is the
/// transient state for an in-flight trail; terminal states carry the
/// answer (`Resolved`), the unresolved violations
/// (`ClarificationExhausted`), or the reason for early termination
/// (`Aborted`, `TimedOut`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AdjudicationOutcome {
    InProgress,
    Resolved {
        answer: serde_json::Value,
    },
    ClarificationExhausted {
        unresolved: Vec<Violation>,
    },
    Aborted {
        reason: String,
    },
    TimedOut,
}

// ---------------------------------------------------------------------------
// Optional integrity chaining (per ADJ07 §"Cryptographic Integrity")
// ---------------------------------------------------------------------------

/// Wraps one appended record in a content-addressed chain. The trail
/// crate does NOT compute the hashes — deployment chooses the hash
/// algorithm and computes hashes at append time. The shape is
/// preserved so reviewers can verify the chain by re-hashing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppendedRecord {
    pub record: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prev_hash: Option<String>,
    pub record_hash: String,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_doc() -> Document {
        Document {
            id: DocumentId::new("doc1"),
            name: "tsa_declaration".into(),
            received_at: "2026-05-11T08:00:00Z".into(),
            normalized_text: "1 carry-on bag, 1 personal item.".into(),
            normalization: NormalizationRecord {
                pipeline: "plain-text-v1".into(),
                version: "1.0.0".into(),
                options: BTreeMap::new(),
            },
            raw_base64: None,
            appended_turns: Vec::new(),
        }
    }

    #[test]
    fn new_trail_is_in_progress_and_minimal() {
        let t = AuditTrail::new(
            AdjudicationId::new("adj-1"),
            "2026-05-11T08:00:00Z",
        );
        assert_eq!(t.adjudication_id.0, "adj-1");
        assert!(matches!(t.outcome, AdjudicationOutcome::InProgress));
        assert!(t.completed_at.is_none());
        assert!(t.documents.is_empty());
        assert_eq!(t.schema_version, "ADJ07-v1");
    }

    #[test]
    fn audit_trail_roundtrips_through_json() {
        let mut t = AuditTrail::new(
            AdjudicationId::new("adj-1"),
            "2026-05-11T08:00:00Z",
        );
        t.documents.push(sample_doc());
        t.outcome = AdjudicationOutcome::Resolved {
            answer: serde_json::json!({"allowed": true}),
        };
        t.completed_at = Some("2026-05-11T08:00:05Z".into());

        let s = serde_json::to_string(&t).unwrap();
        let back: AuditTrail = serde_json::from_str(&s).unwrap();
        assert_eq!(t, back);
    }

    #[test]
    fn outcome_serializes_with_kind_tag() {
        let o = AdjudicationOutcome::Resolved {
            answer: serde_json::json!({"x": 1}),
        };
        let s = serde_json::to_value(&o).unwrap();
        assert_eq!(s["kind"], "resolved");
        assert_eq!(s["answer"]["x"], 1);
    }

    #[test]
    fn timed_out_serializes_as_kind_only() {
        let o = AdjudicationOutcome::TimedOut;
        let s = serde_json::to_value(&o).unwrap();
        assert_eq!(s["kind"], "timed_out");
    }

    #[test]
    fn clarification_exhausted_serializes_with_unresolved() {
        let v = Violation {
            node_id: NodeId::new("n1"),
            pass_name: PassName::Adj02Coverage,
            kind: ClarificationKind::UncoveredSpan,
            detail: serde_json::json!({"range": [3, 10]}),
            triggered_dialogue_turn: None,
            resolved: false,
        };
        let o = AdjudicationOutcome::ClarificationExhausted {
            unresolved: vec![v],
        };
        let s = serde_json::to_string(&o).unwrap();
        assert!(s.contains("\"kind\":\"clarification_exhausted\""));
        assert!(s.contains("\"uncovered_span\""));
    }

    #[test]
    fn pass_name_round_trips_in_snake_case() {
        let cr = CheckerResult {
            pass_name: PassName::Adj04RoundTrip,
            pass_version: "1.0".into(),
            started_at: "2026-05-11T08:00:01Z".into(),
            completed_at: "2026-05-11T08:00:02Z".into(),
            outcome: PassOutcome::Passed,
            violations: Vec::new(),
            telemetry: BTreeMap::new(),
        };
        let s = serde_json::to_string(&cr).unwrap();
        assert!(s.contains("\"pass_name\":\"adj04_round_trip\""));
        let back: CheckerResult = serde_json::from_str(&s).unwrap();
        assert_eq!(back.pass_name, PassName::Adj04RoundTrip);
    }

    #[test]
    fn adj22_typed_quantity_pass_name_round_trips() {
        let cr = CheckerResult {
            pass_name: PassName::Adj22TypedQuantity,
            pass_version: "v0.1".into(),
            started_at: "2026-05-13T08:00:01Z".into(),
            completed_at: "2026-05-13T08:00:02Z".into(),
            outcome: PassOutcome::Failed,
            violations: vec![Violation {
                node_id: NodeId::new("N2"),
                pass_name: PassName::Adj22TypedQuantity,
                kind: ClarificationKind::MissingQuantity,
                detail: serde_json::json!({
                    "literal": "4",
                    "location": [15, 16],
                    "nearby_nodes": ["N2"],
                }),
                triggered_dialogue_turn: None,
                resolved: false,
            }],
            telemetry: BTreeMap::new(),
        };
        let s = serde_json::to_string(&cr).unwrap();
        assert!(s.contains("\"pass_name\":\"adj22_typed_quantity\""));
        assert!(s.contains("\"kind\":\"missing_quantity\""));
        let back: CheckerResult = serde_json::from_str(&s).unwrap();
        assert_eq!(back.pass_name, PassName::Adj22TypedQuantity);
        assert_eq!(back.violations[0].kind, ClarificationKind::MissingQuantity);
    }

    #[test]
    fn violation_omits_empty_optional_fields_in_json() {
        let v = Violation {
            node_id: NodeId::new("n1"),
            pass_name: PassName::Adj03PolarityModality,
            kind: ClarificationKind::AmbiguousPolarity,
            detail: serde_json::Value::Null,
            triggered_dialogue_turn: None,
            resolved: false,
        };
        let s = serde_json::to_value(&v).unwrap();
        assert!(s.get("triggered_dialogue_turn").is_none());
    }

    #[test]
    fn dialogue_turn_round_trips() {
        let t = DialogueTurn {
            turn_id: TurnId(1),
            at: "2026-05-11T08:00:03Z".into(),
            triggering_violation: Some(0),
            rung: DialogueRung::Rung2SecondOpinion,
            question_text: "Did the passenger declare a third bag?".into(),
            response: DialogueResponse {
                source: DialogueResponseSource::Llm,
                text: "No.".into(),
                actor_id: Some("anthropic/claude-haiku".into()),
                model_version: Some("4-5-20251001".into()),
                prompt_version: None,
                prompt_hash: None,
            },
            outcome: DialogueOutcome::Resolved,
        };
        let s = serde_json::to_string(&t).unwrap();
        let back: DialogueTurn = serde_json::from_str(&s).unwrap();
        assert_eq!(back, t);
    }

    #[test]
    fn engine_artifacts_serializes_minimal_form() {
        let e = EngineArtifacts {
            engine_version: "logic-engine 0.4.0".into(),
            search_mode: SearchMode::FindFirst,
            kb_summary: KbSummary {
                fact_count: 3,
                rule_count: 1,
                fact_ids: vec!["f1".into(), "f2".into(), "f3".into()],
                rule_ids: vec!["r1".into()],
                all_certain: true,
            },
            proof_dag: serde_json::json!({"root": "f1"}),
            formula: None,
            wmc_result: None,
            answer: serde_json::json!({"allowed": true}),
        };
        let s = serde_json::to_value(&e).unwrap();
        assert!(s.get("formula").is_none()); // skipped because None
        assert_eq!(s["kb_summary"]["fact_count"], 3);
        assert_eq!(s["search_mode"], "FindFirst");
    }

    #[test]
    fn engine_artifacts_serializes_probabilistic_form() {
        let e = EngineArtifacts {
            engine_version: "logic-engine 0.4.0".into(),
            search_mode: SearchMode::EnumerateAll,
            kb_summary: KbSummary {
                fact_count: 1,
                rule_count: 0,
                fact_ids: vec!["f1".into()],
                rule_ids: Vec::new(),
                all_certain: false,
            },
            proof_dag: serde_json::json!({}),
            formula: Some(BooleanFormula {
                encoding: "d-DNNF".into(),
                payload: serde_json::json!({"nodes": []}),
                fact_vars: [("f1".to_string(), 0)].into_iter().collect(),
                rule_vars: BTreeMap::new(),
            }),
            wmc_result: Some(WmcResult {
                probability: 0.73,
                method: "d-DNNF-eval".into(),
                elapsed_ms: 12,
            }),
            answer: serde_json::Value::Null,
        };
        let s = serde_json::to_value(&e).unwrap();
        assert_eq!(s["formula"]["encoding"], "d-DNNF");
        assert!((s["wmc_result"]["probability"].as_f64().unwrap() - 0.73).abs() < 1e-9);
    }

    #[test]
    fn configuration_round_trips_with_versioned_components() {
        let cfg = Configuration {
            extractor_model: VersionedComponent {
                name: "anthropic/claude-opus".into(),
                version: "4-7-20260301".into(),
                config: [("temperature".into(), serde_json::json!(0.0))]
                    .into_iter()
                    .collect(),
            },
            adversary_sample_rate: 0.25,
            escalation_policy: "strict-cheap-first".into(),
            ..Configuration::default()
        };

        let s = serde_json::to_string(&cfg).unwrap();
        let back: Configuration = serde_json::from_str(&s).unwrap();
        assert_eq!(cfg, back);
        assert_eq!(back.extractor_model.config["temperature"], 0.0);
    }

    #[test]
    fn append_info_records_byte_range() {
        let a = AppendInfo {
            turn_id: TurnId(7),
            start_offset: 100,
            end_offset: 142,
            appended_at: "2026-05-11T08:00:04Z".into(),
        };
        let s = serde_json::to_value(&a).unwrap();
        assert_eq!(s["start_offset"], 100);
        assert_eq!(s["end_offset"], 142);
        assert_eq!(s["turn_id"], 7);
    }

    #[test]
    fn ir_node_is_opaque_payload_at_v01() {
        let n = IrNode {
            id: NodeId::new("n1"),
            document_id: DocumentId::new("doc1"),
            payload: serde_json::json!({
                "kind": "Fact",
                "term": "carry_on(1)",
                "polarity": "Affirmed",
            }),
        };
        let s = serde_json::to_string(&n).unwrap();
        let back: IrNode = serde_json::from_str(&s).unwrap();
        assert_eq!(back.id.0, "n1");
        assert_eq!(back.payload["kind"], "Fact");
    }

    #[test]
    fn appended_record_chains_via_hash() {
        let r1 = AppendedRecord {
            record: serde_json::json!({"step": 1}),
            prev_hash: None,
            record_hash: "abc".into(),
        };
        let r2 = AppendedRecord {
            record: serde_json::json!({"step": 2}),
            prev_hash: Some("abc".into()),
            record_hash: "def".into(),
        };
        // Just shape: the chain is "r2.prev_hash == r1.record_hash"
        assert_eq!(r2.prev_hash.as_deref(), Some(r1.record_hash.as_str()));
    }

    #[test]
    fn schema_version_constant_is_v1() {
        assert_eq!(AuditTrail::CURRENT_SCHEMA_VERSION, "ADJ07-v1");
    }

    #[test]
    fn deserialize_tolerates_missing_optional_fields() {
        // Forward-compatibility: a future producer might omit
        // appended_turns; the schema should accept that.
        let s = r#"{
            "id": "doc1",
            "name": "x",
            "received_at": "2026-05-11T00:00:00Z",
            "normalized_text": "hi",
            "normalization": {
                "pipeline": "plain",
                "version": "1"
            }
        }"#;
        let d: Document = serde_json::from_str(s).unwrap();
        assert!(d.appended_turns.is_empty());
        assert!(d.raw_base64.is_none());
        assert!(d.normalization.options.is_empty());
    }
}
