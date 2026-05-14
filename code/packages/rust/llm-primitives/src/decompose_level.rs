//! # `decompose_level` — level-scoped extraction (ADJ25 / ADJ26 follow-up)
//!
//! Where [`crate::decompose_text`] sends one big "produce a whole IR"
//! prompt (the `v5` flat-IR contract), `decompose_level` sends a
//! short focused prompt scoped to a *single* level boundary of the
//! ADJ25 hierarchical decomposition:
//!
//!  - `Document → Sentence`
//!  - `Sentence → Phrase`
//!  - `Phrase → Claim` (Fact / Uncertainty / Question / Discarded)
//!  - `Fact → TypedComponent` (Quantity / Polarity / Entity / Predicate / Comparator / TimeRef / Modifier)
//!
//! Each level has its OWN system prompt that teaches only the kinds
//! valid at that level + the byte-tiling contract + a worked
//! example. This matches the framework's "small focused tasks"
//! thesis and the per-level retry discipline the orchestrator
//! already uses.
//!
//! ## Why this exists
//!
//! ADJ26's 2026-05-13 foundation bench ran the orchestrator end-to-end
//! against all 5 Ollama models and produced **0/40 usable IRs**. Every
//! cell failed because the orchestrator's per-parent calls routed
//! through `decompose_text`, whose v5 system prompt instructs the
//! model to emit `Fact` / `Rule` / etc. (the flat IR). The
//! orchestrator's correction prompt asked for `Sentence` / `Phrase`
//! / etc. The model followed the dominant system prompt and produced
//! flat-IR kinds the orchestrator's per-level filter then rejected.
//!
//! This primitive fixes the system-prompt mismatch at the root:
//! every level boundary now sees a system prompt that names only the
//! kinds it can return. The user prompt names the parent's text + an
//! optional correction context for retries. The schema is the same
//! `{ "document_id": ..., "nodes": [...] }` shape `decompose_text`
//! returns, so the orchestrator's existing JSON-to-IRNode parsing
//! works unchanged.

use llm_gateway::{
    CompletionRequest, JsonSchema, LlmClient, Message, MessageContent, Role as MsgRole,
};

use crate::{
    fingerprint_prompt, GatewayConfig, LlmCallRecord, PrimitiveError, Role,
};

/// Which level boundary the call is scoped to. Mirrors
/// `adjudication_coverage::DecompLevel` and
/// `adjudication_clarification::DecompositionLevel` 1:1; defined
/// locally so `llm-primitives` does not take a dependency on the
/// higher-level crates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecomposeLevel {
    DocumentToSentence,
    SentenceToPhrase,
    PhraseToClaim,
    FactToTypedComponent,
}

/// Stable version of the per-level prompt templates. Bumping this is
/// an audit-trail-affecting change; the same string flows into
/// `LlmCallRecord::prompt_version` for replay.
pub const DECOMPOSE_LEVEL_PROMPT_VERSION: &str = "decompose-level-v1";

/// Inputs to [`decompose_level`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecomposeLevelRequest {
    /// Stable identifier the framework attaches to this document.
    /// The LLM is instructed to copy it into
    /// `ir_document.document_id`.
    pub document_id: String,
    /// Which level boundary the call decomposes.
    pub level: DecomposeLevel,
    /// The PARENT's source text — not the whole document. Spans in
    /// the response are zero-based offsets within this string; the
    /// orchestrator translates to document-absolute on splice.
    pub parent_text: String,
    /// Optional correction context for retries (prior attempt JSON
    /// + a description of the gap). When `None`, this is an initial
    /// dispatch.
    pub correction_context: Option<String>,
    /// Optional ancestor-chain context for disambiguation. Rendered
    /// in a separate "Surrounding text" block; the model is told
    /// not to decompose it.
    pub ancestor_context: Option<String>,
}

/// Outcome of one [`decompose_level`] call.
#[derive(Debug, Clone, PartialEq)]
pub struct DecomposeLevelResponse {
    pub ir_document: serde_json::Value,
    pub structural_ok: bool,
    pub call_record: LlmCallRecord,
}

// ---------------------------------------------------------------------------
// Per-level system prompts
// ---------------------------------------------------------------------------

const SENTENCE_PROMPT: &str = "You break passages of text into NATURAL-LANGUAGE SENTENCES.\n\
\n\
INPUT: a passage of plain text.\n\
\n\
OUTPUT: a JSON object with `document_id` and a `nodes` array. Each\n\
node represents one sentence (or one chunk you discard). Allowed\n\
`kind` values, and ONLY these:\n\
\n\
  - `Sentence` — a natural-language sentence (declarative,\n\
    interrogative, imperative). Most output nodes will be this.\n\
  - `Discarded` — a chunk of text that is not a sentence: a heading,\n\
    a bullet marker, document metadata, a salutation. Use this for\n\
    bytes that don't belong to any sentence. Discarded nodes need a\n\
    `discard_reason` such as `DocumentMetadata` or `NonDomainContent`.\n\
\n\
COVERAGE: Together, the nodes' `source_spans` MUST cover every byte\n\
of the input passage exactly once. No gaps. No overlaps. Whitespace\n\
counts. Punctuation counts. Trailing newlines count.\n\
\n\
SPANS: `source_spans` is an array of `{ \"start\": <byte>, \"end\": <byte> }`.\n\
Offsets are zero-based byte positions within the input passage.\n\
\n\
EVERY node MUST include: `id` (your choice, unique within the response),\n\
`kind` (one of the values above), `term` (any object, e.g.\n\
`{\"atom\": \"x\"}`), `polarity` (`Affirmed` for sentences and discarded\n\
items), `modality` (`Present`), and `source_spans` (as above).\n\
\n\
EXAMPLE:\n\
INPUT (passage): \"Hello world. How are you?\"\n\
OUTPUT: { \"document_id\": \"<copy from input>\", \"nodes\": [\n\
  { \"id\": \"S1\", \"kind\": \"Sentence\", \"term\": {\"atom\":\"greeting\"},\n\
    \"polarity\": \"Affirmed\", \"modality\": \"Present\",\n\
    \"source_spans\": [{\"start\": 0, \"end\": 13}] },\n\
  { \"id\": \"S2\", \"kind\": \"Sentence\", \"term\": {\"atom\":\"question\"},\n\
    \"polarity\": \"Affirmed\", \"modality\": \"Present\",\n\
    \"source_spans\": [{\"start\": 13, \"end\": 25}] }\n\
] }\n\
\n\
Respond with the JSON object only. No prose, no markdown, no backticks.";

const PHRASE_PROMPT: &str = "You break SENTENCES into PHRASES — coherent sub-sentence chunks.\n\
\n\
INPUT: one sentence.\n\
\n\
OUTPUT: a JSON object with `document_id` and a `nodes` array. Each\n\
node represents one phrase. Allowed `kind` values, and ONLY these:\n\
\n\
  - `Phrase` — a coherent meaning-bearing chunk. A phrase is the unit\n\
    that contributes ONE claim. A short sentence may be one phrase;\n\
    a long one may be several.\n\
  - `Discarded` — a chunk that doesn't carry meaning: pleasantries\n\
    (`\"please\"`, `\"thank you\"`), filler, structural punctuation\n\
    bridging two phrases. Discarded nodes need a `discard_reason`.\n\
\n\
COVERAGE: Together, the nodes' `source_spans` MUST cover every byte\n\
of the input sentence exactly once. No gaps. No overlaps.\n\
\n\
SPANS: `source_spans` is an array of `{ \"start\": <byte>, \"end\": <byte> }`.\n\
Offsets are zero-based byte positions WITHIN THE INPUT SENTENCE,\n\
not the whole document.\n\
\n\
EVERY node MUST include: `id`, `kind`, `term`, `polarity`\n\
(`Affirmed`), `modality` (`Present`), and `source_spans`.\n\
\n\
EXAMPLE:\n\
INPUT (sentence): \"1 carry-on bag, matches.\"\n\
OUTPUT: { \"document_id\": \"<copy from input>\", \"nodes\": [\n\
  { \"id\": \"P1\", \"kind\": \"Phrase\", \"term\": {\"atom\":\"bag_count\"},\n\
    \"polarity\": \"Affirmed\", \"modality\": \"Present\",\n\
    \"source_spans\": [{\"start\": 0, \"end\": 16}] },\n\
  { \"id\": \"P2\", \"kind\": \"Phrase\", \"term\": {\"atom\":\"item\"},\n\
    \"polarity\": \"Affirmed\", \"modality\": \"Present\",\n\
    \"source_spans\": [{\"start\": 16, \"end\": 24}] }\n\
] }\n\
\n\
Respond with the JSON object only.";

const CLAIM_PROMPT: &str = "You break PHRASES into CLAIMS.\n\
\n\
INPUT: one phrase.\n\
\n\
OUTPUT: a JSON object with `document_id` and a `nodes` array. Each\n\
node represents one claim. Allowed `kind` values, and ONLY these:\n\
\n\
  - `Fact` — an assertion the phrase makes about the world. Use this\n\
    when the phrase commits to a definite claim.\n\
  - `Uncertainty` — the phrase admits or implies the model isn't\n\
    sure. Set `polarity` to `Uncertain`.\n\
  - `Question` — the phrase is interrogative (asks a question).\n\
  - `Discarded` — the phrase has no meaningful claim (pure filler).\n\
    Discarded nodes need a `discard_reason`.\n\
\n\
COVERAGE: the nodes' `source_spans` MUST cover every byte of the\n\
input phrase exactly once.\n\
\n\
SPANS: offsets are zero-based byte positions WITHIN THE INPUT PHRASE.\n\
\n\
EVERY node MUST include: `id`, `kind`, `term`, `polarity`\n\
(`Affirmed` for Fact; `Uncertain` for Uncertainty; `Affirmed` for\n\
Question and Discarded), `modality` (`Present`), and `source_spans`.\n\
\n\
EXAMPLE:\n\
INPUT (phrase): \"1 carry-on bag\"\n\
OUTPUT: { \"document_id\": \"<copy from input>\", \"nodes\": [\n\
  { \"id\": \"F1\", \"kind\": \"Fact\", \"term\": {\"atom\":\"declaration\"},\n\
    \"polarity\": \"Affirmed\", \"modality\": \"Present\",\n\
    \"source_spans\": [{\"start\": 0, \"end\": 14}] }\n\
] }\n\
\n\
Respond with the JSON object only.";

const TYPED_COMPONENT_PROMPT: &str = "You break FACTS into TYPED COMPONENTS — the structured slots of a claim.\n\
\n\
INPUT: one Fact's text.\n\
\n\
OUTPUT: a JSON object with `document_id` and a `nodes` array. Each\n\
node represents one typed component. Allowed `kind` values, and\n\
ONLY these:\n\
\n\
  - `Quantity` — a numerical measurement. `term` is\n\
    `{\"functor\": \"quantity\", \"args\": [{\"num\": <value>}, {\"atom\": \"<unit>\"}]}`.\n\
    Every numerical literal in the Fact MUST surface as a `Quantity`.\n\
  - `Polarity` — a negation/affirmation slot. Use when the Fact\n\
    contains cues like \"no\", \"not\", \"denies\". `term` is an atom\n\
    `{\"atom\": \"denied\"}` or `{\"atom\": \"affirmed\"}`.\n\
  - `Entity` — a named or referential noun phrase. `term` is\n\
    `{\"atom\": \"<single_word>\"}` or `{\"atom\": \"<two_word_compound>\"}`.\n\
  - `Predicate` — the verb/relation. `term` is `{\"atom\": \"<verb>\"}`.\n\
  - `Comparator` — an operator. `term` is `{\"atom\": \"<op>\"}` where\n\
    `<op>` is one of `Eq`, `Lt`, `Le`, `Gt`, `Ge`, `Ne`.\n\
  - `TimeRef` — a date, duration, or temporal phrase.\n\
  - `Modifier` — adjective/adverb refinement (\"strike-anywhere\",\n\
    \"disposable\").\n\
\n\
COVERAGE: the nodes' `source_spans` MUST cover every byte of the\n\
Fact's text exactly once.\n\
\n\
NO FLATTENING: numerical literals MUST appear as `Quantity`\n\
components, NOT inside atom names. `battery_50_wh` is REJECTED — the\n\
`50` must be a `Quantity(50, wh)` slot. Atom names like\n\
`pocket_knife_blade_length` that string together three or more\n\
source words are also REJECTED.\n\
\n\
SPANS: offsets are zero-based byte positions WITHIN THE INPUT\n\
FACT'S TEXT.\n\
\n\
EVERY node MUST include: `id`, `kind`, `term`, `polarity`\n\
(`Affirmed`), `modality` (`Present`), and `source_spans`.\n\
\n\
EXAMPLE:\n\
INPUT (fact): \"200 Wh battery\"\n\
OUTPUT: { \"document_id\": \"<copy from input>\", \"nodes\": [\n\
  { \"id\": \"T1\", \"kind\": \"Quantity\",\n\
    \"term\": {\"functor\": \"quantity\", \"args\": [{\"num\": 200}, {\"atom\": \"wh\"}]},\n\
    \"polarity\": \"Affirmed\", \"modality\": \"Present\",\n\
    \"source_spans\": [{\"start\": 0, \"end\": 6}] },\n\
  { \"id\": \"T2\", \"kind\": \"Entity\", \"term\": {\"atom\": \"battery\"},\n\
    \"polarity\": \"Affirmed\", \"modality\": \"Present\",\n\
    \"source_spans\": [{\"start\": 7, \"end\": 14}] }\n\
] }\n\
\n\
Respond with the JSON object only.";

fn system_prompt_for(level: DecomposeLevel) -> &'static str {
    match level {
        DecomposeLevel::DocumentToSentence => SENTENCE_PROMPT,
        DecomposeLevel::SentenceToPhrase => PHRASE_PROMPT,
        DecomposeLevel::PhraseToClaim => CLAIM_PROMPT,
        DecomposeLevel::FactToTypedComponent => TYPED_COMPONENT_PROMPT,
    }
}

const RESPONSE_SCHEMA: &str = r#"{
    "type": "object",
    "required": ["document_id", "nodes"],
    "properties": {
        "document_id": { "type": "string", "minLength": 1 },
        "nodes":       { "type": "array",  "minItems": 0 }
    },
    "additionalProperties": true
}"#;

/// Defense-in-depth sanitizer for prompt-time string interpolation.
///
/// Strips ASCII / Unicode control characters except `\n`, plus
/// Unicode bidi-override / zero-width characters that visually
/// reorder text without changing bytes
/// (`U+200B..=U+200F`, `U+202A..=U+202E`, `U+2066..=U+2069`).
/// Truncates by walking back to a UTF-8 char boundary so
/// `String::truncate` can't panic on multi-byte chars.
///
/// The orchestrator/clarification layer already sanitizes its
/// inputs, but `decompose_level` is a `pub fn` — any future caller
/// that bypasses the clarification layer must not re-introduce the
/// exposure. The sanitizer is cheap (single linear pass) so it's
/// safe to apply unconditionally.
fn sanitize_for_prompt(s: &str, max_len: usize) -> String {
    fn keep_char(c: char) -> bool {
        match c {
            '\n' => true,
            '\u{200B}'..='\u{200F}' => false,
            '\u{202A}'..='\u{202E}' => false,
            '\u{2066}'..='\u{2069}' => false,
            c => !c.is_control(),
        }
    }
    let mut cleaned: String = s.chars().filter(|c| keep_char(*c)).collect();
    if cleaned.len() > max_len {
        let mut cut = max_len;
        while cut > 0 && !cleaned.is_char_boundary(cut) {
            cut -= 1;
        }
        cleaned.truncate(cut);
        cleaned.push_str("…");
    }
    cleaned
}

fn build_user_text(req: &DecomposeLevelRequest) -> String {
    let doc_id = sanitize_for_prompt(&req.document_id, 256);
    let src = sanitize_for_prompt(&req.parent_text, 4_096);
    let mut s = format!(
        "DOCUMENT_ID: {doc_id}\n\nINPUT:\n{src}\n",
        doc_id = doc_id,
        src = src,
    );
    if let Some(anc) = &req.ancestor_context {
        let anc_safe = sanitize_for_prompt(anc, 4_096);
        s.push_str(&format!(
            "\nSURROUNDING TEXT (for context only — do NOT decompose this):\n{anc}\n",
            anc = anc_safe,
        ));
    }
    if let Some(corr) = &req.correction_context {
        let corr_safe = sanitize_for_prompt(corr, 8_192);
        s.push_str(&format!(
            "\nFRAMEWORK CHECKER FEEDBACK on a previous attempt:\n{corr}\n",
            corr = corr_safe,
        ));
    }
    s.push_str("\nReturn the JSON object now.");
    s
}

fn build_completion_request(
    client: &dyn LlmClient,
    req: &DecomposeLevelRequest,
) -> CompletionRequest {
    CompletionRequest {
        model: client.identity().model_family.clone(),
        system: Some(system_prompt_for(req.level).to_string()),
        messages: vec![Message {
            role: MsgRole::User,
            content: MessageContent::Text(build_user_text(req)),
        }],
        // Deterministic by default. Deployments override at the
        // gateway layer if they want sampling.
        temperature: 0.0,
        // Per-level responses are MUCH smaller than the v5 whole-doc
        // response (typically 2-8 children per call). A 2048 cap is
        // generous; the truncation-retry helper doubles up to
        // MAX_TOKENS_CEILING anyway.
        max_tokens: Some(2048),
        stop_sequences: Vec::new(),
        seed: None,
        metadata: Default::default(),
    }
}

fn structural_ok(v: &serde_json::Value, expected_doc_id: &str) -> bool {
    v.get("document_id")
        .and_then(|d| d.as_str())
        .is_some_and(|s| s == expected_doc_id)
        && v.get("nodes").is_some_and(|n| n.is_array())
}

/// Decompose one level boundary via a focused, level-aware LLM call.
///
/// Looks up `Role::Extractor` on the gateway; returns
/// [`PrimitiveError::NoClientForRole`] when absent. The system
/// prompt is selected by `req.level`; the user message includes the
/// parent text, any ancestor context, and any correction context
/// from a prior retry attempt.
pub fn decompose_level(
    req: &DecomposeLevelRequest,
    gateway: &GatewayConfig,
) -> Result<DecomposeLevelResponse, PrimitiveError> {
    let client =
        gateway
            .client(Role::Extractor)
            .ok_or(PrimitiveError::NoClientForRole {
                role: Role::Extractor,
            })?;

    let completion_req = build_completion_request(client, req);
    let prompt_hash = fingerprint_prompt(&completion_req);

    let schema = JsonSchema {
        name: "DecomposedLevel".to_string(),
        schema_json: RESPONSE_SCHEMA.to_string(),
    };

    let json_resp = crate::complete_json_with_truncation_retry(client, completion_req, &schema)
        .map_err(PrimitiveError::Gateway)?;

    if !json_resp.parsed.is_object() {
        return Err(PrimitiveError::ValidationExhausted {
            last_response: json_resp.raw_text,
            last_error: "decompose_level response is not a JSON object".to_string(),
            attempts: 1,
        });
    }

    let structural = structural_ok(&json_resp.parsed, &req.document_id);

    let call_record = LlmCallRecord {
        primitive: "decompose_level".to_string(),
        role: Role::Extractor.as_str().to_string(),
        prompt_version: DECOMPOSE_LEVEL_PROMPT_VERSION.to_string(),
        prompt_hash,
        provider: json_resp.provider_id,
        usage: json_resp.usage,
        finish_reason: llm_gateway::FinishReason::Stop,
        latency_ms: json_resp.latency_ms,
        cost_usd: 0.0,
    };

    Ok(DecomposeLevelResponse {
        ir_document: json_resp.parsed,
        structural_ok: structural,
        call_record,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_prompt_selected_by_level() {
        // The four levels must map to distinct system prompts.
        let s1 = system_prompt_for(DecomposeLevel::DocumentToSentence);
        let s2 = system_prompt_for(DecomposeLevel::SentenceToPhrase);
        let s3 = system_prompt_for(DecomposeLevel::PhraseToClaim);
        let s4 = system_prompt_for(DecomposeLevel::FactToTypedComponent);
        assert_ne!(s1, s2);
        assert_ne!(s2, s3);
        assert_ne!(s3, s4);
        // Each prompt names its level's primary kind verbatim.
        assert!(s1.contains("Sentence"));
        assert!(s2.contains("Phrase"));
        assert!(s3.contains("Fact"));
        assert!(s4.contains("Quantity"));
    }

    #[test]
    fn system_prompts_are_compact() {
        // The point of per-level prompts is being SHORT. v5 was
        // ~14k characters; per-level prompts must each stay under
        // 4k characters as a regression guard.
        for level in [
            DecomposeLevel::DocumentToSentence,
            DecomposeLevel::SentenceToPhrase,
            DecomposeLevel::PhraseToClaim,
            DecomposeLevel::FactToTypedComponent,
        ] {
            let p = system_prompt_for(level);
            assert!(
                p.len() < 4_096,
                "system prompt for {:?} is {} chars; expected < 4096",
                level,
                p.len()
            );
        }
    }

    #[test]
    fn typed_component_prompt_teaches_no_flattening() {
        let p = system_prompt_for(DecomposeLevel::FactToTypedComponent);
        assert!(p.contains("NO FLATTENING"));
        assert!(p.contains("battery_50_wh"));
        assert!(p.contains("pocket_knife_blade_length"));
    }

    #[test]
    fn user_text_embeds_correction_context_when_present() {
        let req = DecomposeLevelRequest {
            document_id: "doc1".into(),
            level: DecomposeLevel::PhraseToClaim,
            parent_text: "1 carry-on bag".into(),
            correction_context: Some("byte 0 wasn't covered".into()),
            ancestor_context: None,
        };
        let t = build_user_text(&req);
        assert!(t.contains("1 carry-on bag"));
        assert!(t.contains("byte 0 wasn't covered"));
        assert!(t.contains("CHECKER FEEDBACK"));
    }

    #[test]
    fn user_text_skips_correction_when_absent() {
        let req = DecomposeLevelRequest {
            document_id: "doc1".into(),
            level: DecomposeLevel::DocumentToSentence,
            parent_text: "hello".into(),
            correction_context: None,
            ancestor_context: None,
        };
        let t = build_user_text(&req);
        assert!(!t.contains("CHECKER FEEDBACK"));
    }

    #[test]
    fn user_text_renders_ancestor_context_when_present() {
        let req = DecomposeLevelRequest {
            document_id: "doc1".into(),
            level: DecomposeLevel::FactToTypedComponent,
            parent_text: "200 Wh".into(),
            correction_context: None,
            ancestor_context: Some("lithium battery, 200 Wh.".into()),
        };
        let t = build_user_text(&req);
        assert!(t.contains("SURROUNDING TEXT"));
        assert!(t.contains("lithium battery, 200 Wh."));
    }

    #[test]
    fn prompt_version_constant_is_stable() {
        assert_eq!(DECOMPOSE_LEVEL_PROMPT_VERSION, "decompose-level-v1");
    }

    #[test]
    fn sanitizer_strips_control_and_bidi_chars() {
        // U+202E is the canonical bidi-override injection vector.
        assert_eq!(
            sanitize_for_prompt("hello\u{202E}world", 4096),
            "helloworld"
        );
        // U+200B (zero-width space).
        assert_eq!(sanitize_for_prompt("a\u{200B}b", 4096), "ab");
        // ASCII C0 control chars.
        assert_eq!(sanitize_for_prompt("a\x01\x02b", 4096), "ab");
        // Newlines are preserved (intentional).
        assert_eq!(sanitize_for_prompt("a\nb", 4096), "a\nb");
    }

    #[test]
    fn sanitizer_truncates_at_utf8_boundary() {
        // CJK chars are 3 bytes each in UTF-8. Truncate mid-codepoint
        // would panic in String::truncate; the boundary walk prevents it.
        let s: String = std::iter::repeat('日').take(10).collect();
        let cleaned = sanitize_for_prompt(&s, 16);
        assert!(cleaned.ends_with('…'));
        assert!(std::str::from_utf8(cleaned.as_bytes()).is_ok());
    }

    #[test]
    fn user_text_sanitizes_caller_supplied_strings() {
        let req = DecomposeLevelRequest {
            document_id: "doc\u{202E}1".into(),
            level: DecomposeLevel::DocumentToSentence,
            parent_text: "hello\x01world".into(),
            correction_context: Some("gap\u{200B}info".into()),
            ancestor_context: Some("around\u{2066}text".into()),
        };
        let t = build_user_text(&req);
        // Every category-Cc / bidi codepoint stripped.
        assert!(!t.contains('\u{202E}'));
        assert!(!t.contains('\x01'));
        assert!(!t.contains('\u{200B}'));
        assert!(!t.contains('\u{2066}'));
        // Useful content survived.
        assert!(t.contains("doc1"));
        assert!(t.contains("helloworld"));
        assert!(t.contains("gapinfo"));
        assert!(t.contains("aroundtext"));
    }
}
