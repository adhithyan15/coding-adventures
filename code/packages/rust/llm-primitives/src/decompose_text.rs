//! # `decompose_text` — the headline extraction primitive
//!
//! Fifth concrete primitive from
//! [LM00b §"decompose_text"](../../../specs/LM00b-llm-primitives.md).
//! Given a source document and a domain hint, produce a hierarchical
//! IR document (per ADJ01 v2): a tree of typed nodes with byte-offset
//! spans that tile the input. The pipeline runs `check_coverage` and
//! `check_propagation` against the result.
//!
//! ## What v0.6 ships
//!
//! - **Builder + LLM call + audit trail**. The primitive constructs
//!   a `complete_json` request against `Role::Extractor`, hashes the
//!   prompt for the audit trail, and returns the LLM's parsed JSON
//!   plus the call record.
//! - **Coverage probe**. A lightweight structural check on the
//!   response: does it look like an IR document? (`{ "document_id":
//!   ..., "nodes": [...] }`.) Full ADJ01 well-formedness lives in
//!   `adjudication_ir::validate` and is the consumer's job — keeping
//!   it out of this primitive avoids a circular dependency between
//!   `llm-primitives` and (someday)
//!   `adjudication-ir`-with-serde-derives.
//!
//! ## Why the response is opaque `serde_json::Value`
//!
//! ADJ01 v2's `IRNode` doesn't yet derive `Serialize` /
//! `Deserialize`. A future minor version will swap
//! [`DecomposeTextResponse::ir_document`] from `serde_json::Value` to
//! a typed `adjudication_ir::IRDocument` — the on-wire shape stays
//! the same, only the static type changes. The same pattern other
//! primitives use (`RenderNodeRequest::node_description`,
//! `JudgePlausibilityRequest::domain_hint`).
//!
//! ## Why no retry-with-correction loop yet
//!
//! The LM00b spec describes a "retry up to N times with a correction
//! prompt on failure" pattern. v0.6 of this primitive is the
//! single-shot bottom layer of that loop. The retry harness can wrap
//! it in a follow-up — it owns the retry policy and the count, the
//! primitive owns the single LLM round-trip.

use llm_gateway::{
    CompletionRequest, JsonSchema, LlmClient, Message, MessageContent, Role as MsgRole,
};

use crate::{
    fingerprint_prompt, GatewayConfig, LlmCallRecord, PrimitiveError, Role, DECOMPOSE_TEXT_PROMPT_VERSION,
};

/// Inputs to [`decompose_text`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecomposeTextRequest {
    /// Stable identifier the framework attaches to this document. The
    /// LLM is instructed to copy it into `ir_document.document_id`,
    /// so the audit trail can join the IR back to the source.
    pub document_id: String,
    /// The normalized source text. IR-node `source_spans` are byte
    /// offsets into this string.
    pub source_text: String,
    /// Free-text domain hint (`"clinical-note"`, `"tsa-declaration"`,
    /// `"legal-contract"`, …). Per LM00b spec the type will become a
    /// `DomainHints` enum in a follow-up.
    pub domain_hint: String,
    /// Optional ISO language code (`"en"`, `"es"`, …). When absent,
    /// the LLM is instructed to auto-detect from the source text.
    pub language_hint: Option<String>,
}

/// Outcome of one [`decompose_text`] call. v0.6 returns the IR
/// document as an opaque `serde_json::Value`; callers run their own
/// `adjudication_ir::validate` against it.
#[derive(Debug, Clone, PartialEq)]
pub struct DecomposeTextResponse {
    /// The LLM's IR output, parsed as JSON. Shape matches ADJ01 v2
    /// (`{ "document_id": ..., "nodes": [...] }`). A future version
    /// will swap this to a typed `adjudication_ir::IRDocument`.
    pub ir_document: serde_json::Value,
    /// Quick structural sanity check: `true` iff the response has
    /// `document_id` (string) plus a `nodes` array. Does NOT
    /// guarantee ADJ01 well-formedness — that's `validate`'s job.
    pub structural_ok: bool,
    pub call_record: LlmCallRecord,
}

const SYSTEM_PROMPT: &str = "\
You are a precise document-to-IR extractor. Given a SOURCE document \
and a DOMAIN hint, produce a JSON document with TWO top-level \
collections — `nodes` (typed claims) and `edges` (typed relationships) \
— in this exact shape:\n\
\n\
{\n\
  \"document_id\": \"<copy DOCUMENT_ID verbatim>\",\n\
  \"nodes\": [\n\
    {\n\
      \"id\":           \"N1\",\n\
      \"kind\":         \"Fact\",\n\
      \"term\":         { \"functor\": \"<predicate>\", \"args\": [ { \"atom\": \"<value>\" } ] },\n\
      \"polarity\":     \"Affirmed\",\n\
      \"modality\":     \"Present\",\n\
      \"source_spans\": [ { \"start\": 0, \"end\": 16 } ]\n\
    },\n\
    ...\n\
  ],\n\
  \"edges\": [\n\
    {\n\
      \"id\":           \"E1\",\n\
      \"source\":       \"<source NodeId>\",\n\
      \"target\":       \"<target NodeId>\",\n\
      \"relation\":     \"<relation name>\",\n\
      \"polarity\":     \"Affirmed\",\n\
      \"modality\":     \"Present\",\n\
      \"source_spans\": [ { \"start\": 14, \"end\": 16 } ]\n\
    },\n\
    ...\n\
  ]\n\
}\n\
\n\
Worked example. SOURCE: `\"1 carry-on bag, matches.\"` (24 bytes).\n\
Correct output:\n\
\n\
{\n\
  \"document_id\": \"<doc-id>\",\n\
  \"nodes\": [\n\
    { \"id\": \"N1\", \"kind\": \"Fact\",\n\
      \"term\": { \"functor\": \"carry_on\", \"args\": [ { \"atom\": \"1\" } ] },\n\
      \"polarity\": \"Affirmed\", \"modality\": \"Present\",\n\
      \"source_spans\": [ { \"start\": 0, \"end\": 14 } ] },\n\
    { \"id\": \"N2\", \"kind\": \"Fact\",\n\
      \"term\": { \"functor\": \"prohibited\", \"args\": [ { \"atom\": \"matches\" } ] },\n\
      \"polarity\": \"Affirmed\", \"modality\": \"Present\",\n\
      \"source_spans\": [ { \"start\": 16, \"end\": 23 } ] },\n\
    { \"id\": \"S1\", \"kind\": \"Section\",\n\
      \"term\": { \"functor\": \"sentence\", \"args\": [] },\n\
      \"polarity\": \"Affirmed\", \"modality\": \"Present\",\n\
      \"source_spans\": [ { \"start\": 14, \"end\": 16 }, { \"start\": 23, \"end\": 24 } ] },\n\
    { \"id\": \"Q1\", \"kind\": \"Query\",\n\
      \"term\": { \"functor\": \"compliant\", \"args\": [ { \"atom\": \"passenger\" } ] },\n\
      \"polarity\": \"Affirmed\", \"modality\": \"Present\",\n\
      \"source_spans\": [] }\n\
  ],\n\
  \"edges\": [\n\
    { \"id\": \"E1\", \"source\": \"S1\", \"target\": \"N1\",\n\
      \"relation\": \"Contains\",\n\
      \"polarity\": \"Affirmed\", \"modality\": \"Present\",\n\
      \"source_spans\": [] },\n\
    { \"id\": \"E2\", \"source\": \"S1\", \"target\": \"N2\",\n\
      \"relation\": \"Contains\",\n\
      \"polarity\": \"Affirmed\", \"modality\": \"Present\",\n\
      \"source_spans\": [] }\n\
  ]\n\
}\n\
\n\
The Section node owns the connective bytes (the `, ` between N1 and \
N2, and the trailing `.`). N1 owns the `1 carry-on bag` substring; \
N2 owns the `matches` substring. The two `Contains` edges record \
that the Section groups N1 and N2 — semantic structure that the \
checker passes use. Coverage adds up: N1[0,14] + S1[14,16] + \
N2[16,23] + S1[23,24] = [0,24).\n\
\n\
RULES (every rule is mandatory, no exceptions):\n\
\n\
## NODE rules\n\
\n\
1. **Flat `nodes` array.** Do NOT nest nodes inside a `children` \
field. Every node is a top-level entry.\n\
2. **Field names are exact.** Use `kind` (not `node_type`), `term` \
(not `text`), `source_spans` (not `spans`). Stick to the example.\n\
3. **`kind` is one of**: `Fact`, `Query`, `Uncertainty`, `Rule`, \
`Exception`, `Discarded`, `Section`, `Entity`.\n\
   * `Section` is a structural unit (paragraph, sentence, table, \
row, cell, heading). Its `source_spans` cover ONLY the meta-text \
of the unit (heading, numbering, delimiters), NOT the content. \
The content lives in other nodes connected by `Contains` edges.\n\
   * `Entity` is a deduplicated reference target. When the same \
atom is mentioned multiple times, emit ONE Entity node (with \
empty `source_spans` is acceptable since it's synthesized) and \
emit `Mentions` edges from each mention site to it.\n\
4. **`polarity` is one of**: `Affirmed`, `Denied`, `Uncertain`, \
`Inherit`. Default to `Affirmed`. `Inherit` is valid ONLY on nodes \
that have at least one incoming `Contains` edge (the polarity is \
inherited from the parent Section).\n\
5. **`modality` is one of**: `Present`, `Past`, `Future`, \
`Hypothetical`, `FamilyHistory`, `RuledOut`, `Conditional`, \
`Inherit`. Default to `Present`.\n\
6. **`term`** is either `{\"atom\": \"name\"}` for atomic claims or \
`{\"functor\": \"pred\", \"args\": [...]}` for compound claims. Args \
recursively use the same term shape.\n\
7. **Query nodes have empty `source_spans: []`** — they're \
synthesized questions. Entity nodes MAY have empty `source_spans` \
when synthesized. Every other kind MUST have non-empty \
`source_spans`.\n\
\n\
## EDGE rules\n\
\n\
8. **`edges` is required**, at minimum an empty array `[]` for \
trivial documents. Every relationship between nodes MUST be \
expressed as an explicit edge; do not encode relationships through \
node order, term-argument nesting, or metadata strings.\n\
9. **`relation` is one of** (closed set):\n\
   * Structural: `Contains`, `Precedes`, `Heading`\n\
   * Identity: `Mentions`, `SameAs`, `Refers`\n\
   * Rule modification: `Excepts`, `Refines`, `Generalizes`, \
`Supersedes`, `Restricts`\n\
   * Application: `AppliesTo`, `AppliesWhen`, `Concludes`\n\
   * Provenance: `DerivedFrom`, `JustifiedBy`, `ElicitedFrom`\n\
   * Tabular: `RowOf`, `ColumnOf`, `HeaderOf`, `CellOf`\n\
   * Temporal: `Before`, `After`, `During`, `EffectiveAt`, \
`SupersededAt`\n\
   * Cross-source: `ConflictsWith`, `Confirms`, `DependsOn`\n\
   * Discourse: `Defines`, `Restates`, `Cites`\n\
   * Refinement: `Clarifies`\n\
   If none of these fit, do not invent a name — use `Refers` and \
attach metadata.\n\
10. **Edge `source_spans` cover the TEXTUAL MARKER that signals \
the relation** — the word `except`, the phrase `see §5`, the \
comma between list items — NOT the spans of the related nodes. A \
synthesized edge with no textual marker has `source_spans: []`.\n\
11. **`Excepts` edges connect Exception nodes to Rule nodes.** \
Every Exception MUST be the source of at least one `Excepts` edge.\n\
12. **No cycles.** The graph (nodes, edges) MUST be acyclic across \
ALL relations. Specifically: an edge cannot point a node back to \
itself directly or transitively through any other edges.\n\
\n\
## COVERAGE rules\n\
\n\
13. **Spans TILE the source.** The union of all `source_spans` \
across nodes and edges must cover every byte from 0 to \
`len(SOURCE_bytes)` exactly once. No gaps. No overlaps. INCLUDING \
whitespace and punctuation. Choose how to assign each byte: to a \
node (the content) or to an edge (the connective marker).\n\
14. **Spans are byte offsets**, not character indices. `start` and \
`end` are integers; `0 <= start < end <= len(SOURCE_bytes)`. For \
ASCII text byte offsets equal character indices.\n\
15. **Synthesized objects are exempt from tiling**: Query nodes \
with empty spans, Entity nodes with empty spans, and edges with \
empty spans (those without a textual marker) do not contribute to \
the tiling. Use them freely; the validator skips them.\n\
\n\
## Other\n\
\n\
16. **`document_id` is the DOCUMENT_ID from the user message, \
verbatim.**\n\
17. **Every IR document should include at least one Query node** so \
the engine has something to answer.\n\
18. **Punctuation and delimiters can flip meaning — read them \
carefully.** A single comma, period, colon, or quote mark can \
invert the intent of an otherwise-identical string:\n\
   * `\"Let's eat, Bob.\"` — Bob is being invited to a meal.\n\
   * `\"Let's eat Bob.\"` — Bob is the meal.\n\
The bytes differ by one comma; the meaning differs by an order \
of magnitude. Before assigning a `term`, scan the surrounding \
punctuation: commas separating list items vs vocatives; periods \
ending sentences vs abbreviations; quotes scoping a quoted phrase \
vs marking emphasis; colons introducing definitions vs ratios; \
parentheses denoting asides vs grouping. If the punctuation is \
ambiguous or load-bearing, prefer an `Uncertainty` node with \
`polarity: \"Uncertain\"` over a confident guess.\n\
\n\
Respond with the JSON object only. No prose, no markdown, no \
backticks.";

const RESPONSE_SCHEMA: &str = r#"{
    "type": "object",
    "required": ["document_id", "nodes", "edges"],
    "properties": {
        "document_id": { "type": "string", "minLength": 1 },
        "nodes":       { "type": "array",  "minItems": 0 },
        "edges":       { "type": "array",  "minItems": 0 }
    },
    "additionalProperties": true
}"#;

fn build_user_text(req: &DecomposeTextRequest) -> String {
    let lang = req.language_hint.as_deref().unwrap_or("auto-detect");
    format!(
        "DOMAIN: {domain}\nLANGUAGE: {lang}\nDOCUMENT_ID: {doc_id}\n\n\
         SOURCE:\n{src}\n\n\
         Return the JSON IR document now.",
        domain = req.domain_hint,
        lang = lang,
        doc_id = req.document_id,
        src = req.source_text,
    )
}

fn build_completion_request(
    client: &dyn LlmClient,
    req: &DecomposeTextRequest,
) -> CompletionRequest {
    CompletionRequest {
        model: client.identity().model_family.clone(),
        system: Some(SYSTEM_PROMPT.to_string()),
        messages: vec![Message {
            role: MsgRole::User,
            content: MessageContent::Text(build_user_text(req)),
        }],
        // Deterministic by default; deployments override at the
        // gateway layer if they want sampling.
        temperature: 0.0,
        // IR documents can be large; pick a higher cap than other
        // primitives. A long clinical note can easily run to several
        // thousand output tokens.
        max_tokens: Some(8192),
        stop_sequences: Vec::new(),
        seed: None,
        metadata: Default::default(),
    }
}

/// Lightweight structural sanity check. Returns `true` iff the
/// parsed value is an object with a non-empty string `document_id`
/// and an array `nodes`. Full ADJ01 v2 well-formedness is the
/// caller's responsibility.
fn structural_ok(v: &serde_json::Value, expected_doc_id: &str) -> bool {
    v.get("document_id")
        .and_then(|d| d.as_str())
        .is_some_and(|s| s == expected_doc_id)
        && v.get("nodes").is_some_and(|n| n.is_array())
}

/// Extract an IR document from a source text via the LLM. Looks up
/// `Role::Extractor` on the gateway; returns
/// [`PrimitiveError::NoClientForRole`] when absent,
/// [`PrimitiveError::Gateway`] on transport failures,
/// [`PrimitiveError::ValidationExhausted`] when the response is not
/// a parseable JSON object with the required top-level fields.
pub fn decompose_text(
    req: &DecomposeTextRequest,
    gateway: &GatewayConfig,
) -> Result<DecomposeTextResponse, PrimitiveError> {
    let client = gateway
        .client(Role::Extractor)
        .ok_or(PrimitiveError::NoClientForRole {
            role: Role::Extractor,
        })?;

    let completion_req = build_completion_request(client, req);
    let prompt_hash = fingerprint_prompt(&completion_req);

    let schema = JsonSchema {
        name: "IRDocument".to_string(),
        schema_json: RESPONSE_SCHEMA.to_string(),
    };

    let json_resp =
        crate::complete_json_with_truncation_retry(client, completion_req, &schema)
            .map_err(PrimitiveError::Gateway)?;

    if !json_resp.parsed.is_object() {
        return Err(PrimitiveError::ValidationExhausted {
            last_response: json_resp.raw_text,
            last_error: "decompose_text response is not a JSON object".to_string(),
            attempts: 1,
        });
    }

    let structural = structural_ok(&json_resp.parsed, &req.document_id);

    let call_record = LlmCallRecord {
        primitive: "decompose_text".to_string(),
        role: Role::Extractor.as_str().to_string(),
        prompt_version: DECOMPOSE_TEXT_PROMPT_VERSION.to_string(),
        prompt_hash,
        provider: json_resp.provider_id,
        usage: json_resp.usage,
        finish_reason: llm_gateway::FinishReason::Stop,
        latency_ms: json_resp.latency_ms,
        cost_usd: 0.0,
    };

    Ok(DecomposeTextResponse {
        ir_document: json_resp.parsed,
        structural_ok: structural,
        call_record,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use llm_gateway::{
        Capabilities, CompletionJsonResponse, CompletionRequest, JsonSchema, LlmClient, LlmError,
        MockLlmClient, ProviderIdentity, TokenUsage,
    };
    use std::sync::Mutex;

    fn extractor_identity() -> ProviderIdentity {
        ProviderIdentity {
            vendor: "mock".into(),
            model_family: "opus-extractor".into(),
            model_version: "1".into(),
            endpoint: None,
        }
    }

    struct ScriptedJson {
        identity: ProviderIdentity,
        response: Mutex<Option<Result<CompletionJsonResponse, LlmError>>>,
    }

    impl ScriptedJson {
        fn new(value: serde_json::Value) -> Self {
            let raw_text = value.to_string();
            Self {
                identity: extractor_identity(),
                response: Mutex::new(Some(Ok(CompletionJsonResponse {
                    raw_text,
                    parsed: value,
                    schema_valid: true,
                    model: "opus-extractor".into(),
                    usage: TokenUsage {
                        input_tokens: 700,
                        output_tokens: 320,
                        cached_tokens: 0,
                    },
                    provider_id: extractor_identity(),
                    latency_ms: 1842,
                    polyfill_used: false,
                }))),
            }
        }

        fn with_error(err: LlmError) -> Self {
            Self {
                identity: extractor_identity(),
                response: Mutex::new(Some(Err(err))),
            }
        }
    }

    impl LlmClient for ScriptedJson {
        fn identity(&self) -> ProviderIdentity {
            self.identity.clone()
        }
        fn capabilities(&self) -> Capabilities {
            Capabilities::modern_frontier()
        }
        fn complete(
            &self,
            _req: CompletionRequest,
        ) -> Result<llm_gateway::CompletionResponse, LlmError> {
            unreachable!("decompose_text uses complete_json")
        }
        fn complete_json(
            &self,
            _req: CompletionRequest,
            _schema: &JsonSchema,
        ) -> Result<CompletionJsonResponse, LlmError> {
            self.response
                .lock()
                .unwrap()
                .take()
                .expect("ScriptedJson::complete_json called more than once")
        }
    }

    fn req() -> DecomposeTextRequest {
        DecomposeTextRequest {
            document_id: "doc-tsa-001".into(),
            source_text: "1 carry-on bag, 1 personal item.".into(),
            domain_hint: "tsa-declaration".into(),
            language_hint: None,
        }
    }

    fn happy_ir_value() -> serde_json::Value {
        serde_json::json!({
            "document_id": "doc-tsa-001",
            "nodes": [
                {
                    "id": "n1",
                    "kind": "Fact",
                    "term": { "atom": "carry_on(1)" },
                    "source_spans": [{ "start": 0, "end": 14 }],
                },
                {
                    "id": "n2",
                    "kind": "Fact",
                    "term": { "atom": "personal_item(1)" },
                    "source_spans": [{ "start": 16, "end": 32 }],
                }
            ]
        })
    }

    #[test]
    fn missing_extractor_client_returns_no_client_for_role() {
        let g = GatewayConfig::new();
        let err = decompose_text(&req(), &g).unwrap_err();
        match err {
            PrimitiveError::NoClientForRole { role } => assert_eq!(role, Role::Extractor),
            other => panic!("expected NoClientForRole, got {other:?}"),
        }
    }

    #[test]
    fn happy_path_returns_parsed_ir_and_call_record() {
        let mock = ScriptedJson::new(happy_ir_value());
        let g = GatewayConfig::new().with_client(Role::Extractor, Box::new(mock));
        let resp = decompose_text(&req(), &g).unwrap();

        assert!(resp.structural_ok);
        assert_eq!(resp.ir_document["document_id"], "doc-tsa-001");
        assert_eq!(resp.ir_document["nodes"].as_array().unwrap().len(), 2);

        assert_eq!(resp.call_record.primitive, "decompose_text");
        assert_eq!(resp.call_record.role, "extractor");
        assert_eq!(resp.call_record.prompt_version, "decompose-text-v4");
        assert!(!resp.call_record.prompt_hash.is_empty());
        assert_eq!(resp.call_record.usage.input_tokens, 700);
        assert_eq!(resp.call_record.usage.output_tokens, 320);
        assert_eq!(resp.call_record.latency_ms, 1842);
    }

    #[test]
    fn user_message_includes_domain_language_and_document_id() {
        let r = DecomposeTextRequest {
            language_hint: Some("es".into()),
            ..req()
        };
        let text = build_user_text(&r);
        assert!(text.contains("DOMAIN: tsa-declaration"));
        assert!(text.contains("LANGUAGE: es"));
        assert!(text.contains("DOCUMENT_ID: doc-tsa-001"));
        assert!(text.contains("SOURCE:\n1 carry-on bag"));
    }

    #[test]
    fn missing_language_hint_renders_auto_detect() {
        let text = build_user_text(&req());
        assert!(text.contains("LANGUAGE: auto-detect"));
    }

    #[test]
    fn gateway_context_too_large_propagates_as_gateway_variant() {
        // Realistic failure on a very long document.
        let mock = ScriptedJson::with_error(LlmError::ContextTooLarge {
            provider: extractor_identity(),
            requested_tokens: 250_000,
            max_tokens: 200_000,
        });
        let g = GatewayConfig::new().with_client(Role::Extractor, Box::new(mock));
        let err = decompose_text(&req(), &g).unwrap_err();
        assert!(matches!(
            err,
            PrimitiveError::Gateway(LlmError::ContextTooLarge { .. })
        ));
    }

    #[test]
    fn non_object_response_returns_validation_exhausted() {
        let mock = ScriptedJson::new(serde_json::json!(["just", "an", "array"]));
        let g = GatewayConfig::new().with_client(Role::Extractor, Box::new(mock));
        let err = decompose_text(&req(), &g).unwrap_err();
        match err {
            PrimitiveError::ValidationExhausted { last_error, .. } => {
                assert!(last_error.contains("not a JSON object"));
            }
            other => panic!("expected ValidationExhausted, got {other:?}"),
        }
    }

    #[test]
    fn missing_document_id_marks_structural_ok_false() {
        // The response IS an object, but missing `document_id` — the
        // primitive does NOT fail (the LLM may be using a different
        // shape), but signals `structural_ok = false` so the caller
        // knows to retry with a correction prompt.
        let mock = ScriptedJson::new(serde_json::json!({ "nodes": [] }));
        let g = GatewayConfig::new().with_client(Role::Extractor, Box::new(mock));
        let resp = decompose_text(&req(), &g).unwrap();
        assert!(!resp.structural_ok);
    }

    #[test]
    fn wrong_document_id_marks_structural_ok_false() {
        let mock = ScriptedJson::new(serde_json::json!({
            "document_id": "wrong-id",
            "nodes": [],
        }));
        let g = GatewayConfig::new().with_client(Role::Extractor, Box::new(mock));
        let resp = decompose_text(&req(), &g).unwrap();
        assert!(!resp.structural_ok);
    }

    #[test]
    fn nodes_not_array_marks_structural_ok_false() {
        let mock = ScriptedJson::new(serde_json::json!({
            "document_id": "doc-tsa-001",
            "nodes": "this should be an array",
        }));
        let g = GatewayConfig::new().with_client(Role::Extractor, Box::new(mock));
        let resp = decompose_text(&req(), &g).unwrap();
        assert!(!resp.structural_ok);
    }

    #[test]
    fn empty_nodes_array_is_structurally_ok() {
        // Zero nodes is valid IR (e.g., a discarded document). The
        // coverage check will run downstream and decide whether the
        // tiling is correct for the given source.
        let mock = ScriptedJson::new(serde_json::json!({
            "document_id": "doc-tsa-001",
            "nodes": [],
        }));
        let g = GatewayConfig::new().with_client(Role::Extractor, Box::new(mock));
        let resp = decompose_text(&req(), &g).unwrap();
        assert!(resp.structural_ok);
    }

    #[test]
    fn call_record_prompt_hash_matches_built_request() {
        let stub: Box<dyn LlmClient> =
            Box::new(MockLlmClient::new().with_identity(extractor_identity()));
        let cr = build_completion_request(stub.as_ref(), &req());
        let expected_hash = crate::fingerprint_prompt(&cr);

        let mock = ScriptedJson::new(happy_ir_value());
        let g = GatewayConfig::new().with_client(Role::Extractor, Box::new(mock));
        let resp = decompose_text(&req(), &g).unwrap();
        assert_eq!(resp.call_record.prompt_hash, expected_hash);
    }

    #[test]
    fn ir_document_payload_round_trips_unchanged() {
        // The primitive must not mutate the LLM's JSON output. The
        // caller sees exactly what came back, modulo serde_json's
        // canonical representation.
        let payload = happy_ir_value();
        let mock = ScriptedJson::new(payload.clone());
        let g = GatewayConfig::new().with_client(Role::Extractor, Box::new(mock));
        let resp = decompose_text(&req(), &g).unwrap();
        assert_eq!(resp.ir_document, payload);
    }
}
