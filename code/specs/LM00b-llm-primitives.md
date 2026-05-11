# LM00b — LLM Primitives for IR Generation

## Overview

This spec defines the six **typed primitives** that sit between the
framework's checker passes / extractor and the LLM gateway
([`LM00`](LM00-llm-gateway-architecture.md)). Every framework
component that uses an LLM calls one of these primitives; none call
the gateway directly.

The primitives are deliberately narrow. Each is a *pure function from
typed input to typed output* — the LLM is invisible to the caller.
This is the discipline that lets the framework remain
provider-agnostic and lets every LLM call be recorded uniformly in
the audit trail.

The six primitives:

| Primitive | Input | Output | Used by |
|---|---|---|---|
| `decompose_text` | source text + IR schema | `IRDocument` (tree) | extractor |
| `render_node` | IRNode | one-sentence rendering | ADJ04 round-trip |
| `entail` | two texts | bidirectional entailment + scores | ADJ04 NLI |
| `find_contradicting_reading` | source span + IR rendering | `CONCURS` \| alternative reading | ADJ05 adversary |
| `judge_plausibility` | source + IR + alternative | `PLAUSIBLE` \| `IMPLAUSIBLE` | ADJ05 plausibility |
| `extract_rules` | rulebook segment + schema | `[RuleNode]` | ADJ09 rule pipeline |

Each is backed by a versioned prompt template and a JSON-schema-
validated output. Each primitive has retry logic, a deterministic
fingerprint for mock testing, and audit-trail wiring.

## Layer Position

```
   framework consumers (extractor, ADJ02–05, ADJ06, ADJ09)
        │
        ▼
   LM00b primitives          ← this document
        │
        ▼
   LM00  LlmClient trait
        │
        ▼
   LM00a providers
```

## Design Principles

**One primitive per LLM-driven operation in the framework.** The
checker passes and the extractor never assemble prompts directly;
they call primitives. The primitive owns the prompt template, the
output schema, and the post-hoc validation. Centralising this lets
the framework swap models, refine prompts, or upgrade schemas
without touching the checkers.

**Typed output, schema-validated.** Every primitive's output is a
typed structure (an IR document, a node, an entailment verdict). The
output schema is part of the primitive's contract and is recorded
in the audit trail.

**Versioned prompts.** Each primitive carries a `PromptVersion`
identifier. Changing the prompt bumps the version; the audit trail
records which version each call used. Replay matches on version.

**Role-based provider selection.** Each primitive declares a *role*
(extractor / renderer / nli / adversary / plausibility / rule
extractor); the deployment's `GatewayConfig` maps each role to a
concrete `LlmClient`. ADJ05's independence requirement (extractor
and adversary must be different model families) is enforced by the
deployment's role mapping, not by the primitive.

## decompose_text

The headline primitive. Given source text plus an IR schema, returns
a hierarchical IR document (per ADJ01 v2).

```rust
pub struct DecomposeTextRequest {
    pub document_id:   DocumentId,
    pub source_text:   String,
    pub domain_hints:  DomainHints,   // e.g., "clinical-note", "tsa-declaration"
    pub language_hint: Option<String>, // e.g., "es", "ja"; None = auto-detect
}

pub struct DecomposeTextResponse {
    pub ir_document:   IRDocument,
    pub coverage_pass: bool,           // whether the LLM's tree tiles the input
    pub call_record:   LlmCallRecord,
}

pub async fn decompose_text(
    req: DecomposeTextRequest,
    gateway: &GatewayConfig,
) -> Result<DecomposeTextResponse, PrimitiveError>;
```

### Prompt shape

The prompt embeds:
1. A short description of the framework's purpose.
2. The ADJ01 v2 IR schema (JSON Schema, generated from the Rust
   types). Stable; reused across calls; cacheable via prompt
   caching.
3. The controlled vocabularies (DiscardReason, Polarity, Modality).
4. The domain hint (so the LLM knows what's likely meaningful).
5. The source text.
6. Instructions: "produce a hierarchical decomposition that tiles
   the source text. Every byte must be in some leaf. Use TextRun
   nodes to group; use Fact/Query/Uncertainty/Rule/Discarded as
   leaves. Use Inherit for polarity/modality unless overriding."

The JSON schema is parameterised on the document length so spans
are bounded; the LLM is instructed to emit byte offsets, not
character offsets.

### Validation and retry

The response goes through three checks before returning:

1. **JSON Schema validation** (gateway-level).
2. **ADJ01 v2 well-formedness** (call `adjudication_ir::validate`).
3. **Structural coverage** (`ADJ02 v2`'s tree check).

On failure of any check, the primitive retries up to N times
(default 3) with a correction prompt: *"The previous response failed
this check: \<failure detail\>. Please re-emit the IR with the
specific portion of the source you missed."*. After N retries, the
primitive returns `PrimitiveError::ValidationExhausted` with the
last response attached.

### Domain hints

```rust
pub enum DomainHints {
    None,
    Clinical,
    Legal,
    TsaDeclaration,
    LicenseCompatibility,
    Custom { description: String, examples: Vec<String> },
}
```

Domain hints adjust the prompt suffix to mention typical structural
shapes (a clinical note typically has History / Examination /
Assessment sections; a TSA declaration is a flat list). The hint
does **not** change the IR schema; only the LLM's guidance.

## render_node

Render a single IR node (typically a leaf) back into natural language.

```rust
pub struct RenderNodeRequest {
    pub node:          IRNode,
    pub document:      Document,
    pub style:         RenderStyle,
}

pub enum RenderStyle {
    Plain,       // "The patient has chest pain."
    Clinical,    // "Chest pain (acute, severe), affirmed."
    Legal,       // formal-register restatement
}

pub struct RenderNodeResponse {
    pub rendering: String,
    pub call_record: LlmCallRecord,
}

pub async fn render_node(
    req: RenderNodeRequest,
    gateway: &GatewayConfig,
) -> Result<RenderNodeResponse, PrimitiveError>;
```

The rendering is deliberately weak — a faithful but trivial
paraphrase rather than a clever rewrite. Cleverness masks IR loss;
trivial paraphrasing exposes it (per ADJ04 §"Render IR → Natural
Language").

The role is `renderer`. Deployments typically assign this to a small,
cheap model (e.g., Claude Haiku, GPT-4o-mini).

## entail

Bidirectional textual entailment.

```rust
pub struct EntailRequest {
    pub premise:    String,
    pub hypothesis: String,
}

pub struct EntailResponse {
    pub premise_entails_hypothesis: bool,
    pub p_to_h_score:               f32,    // 0..1
    pub hypothesis_entails_premise: bool,
    pub h_to_p_score:               f32,
    pub call_record:                LlmCallRecord,
}

pub async fn entail(
    req: EntailRequest,
    gateway: &GatewayConfig,
) -> Result<EntailResponse, PrimitiveError>;
```

The role is `nli`. Deployments may assign this to a purpose-trained
NLI model (DeBERTa-v3-MNLI and friends) via an Ollama or HF
endpoint, or to a small LLM with an instruction prompt. Per ADJ04,
the NLI model **must** be different from the renderer to avoid the
self-confirmation loop.

The prompt instructs the model to evaluate entailment in **both
directions** independently and report each with a confidence score.

## find_contradicting_reading

The ADJ05 adversary. Given a source span and the IR's rendering,
find the strongest reading of the source that *contradicts* the IR.

```rust
pub struct FindContradictingReadingRequest {
    pub source_span_text: String,
    pub ir_rendered:      String,
    pub domain_hints:     DomainHints,
}

pub enum FindContradictingReadingResponse {
    Concurs,
    Reading { text: String, explanation: String, call_record: LlmCallRecord },
}

pub async fn find_contradicting_reading(
    req: FindContradictingReadingRequest,
    gateway: &GatewayConfig,
) -> Result<FindContradictingReadingResponse, PrimitiveError>;
```

The prompt is **asymmetric** (per ADJ05): *"Assume the extraction is
wrong. Find the strongest reading of this source that contradicts
\<ir_rendered\>. If you cannot find one, say CONCURS."* The asymmetry
is the point — symmetric "review this" prompts produce uninteresting
agreement.

The role is `adversary`. The deployment **must** assign this to a
different model family from the extractor's role; ADJ05 depends on
this for independence. The framework warns at configuration time if
the same family is assigned to both roles.

## judge_plausibility

The ADJ05 plausibility judge. Given the source, the IR, and the
adversary's contradicting reading, decide whether the reading is
plausible.

```rust
pub struct JudgePlausibilityRequest {
    pub source_span_text:   String,
    pub ir_rendered:        String,
    pub adversary_reading:  String,
    pub domain_hints:       DomainHints,
}

pub struct JudgePlausibilityResponse {
    pub plausible:   bool,
    pub reason:      String,
    pub call_record: LlmCallRecord,
}

pub async fn judge_plausibility(
    req: JudgePlausibilityRequest,
    gateway: &GatewayConfig,
) -> Result<JudgePlausibilityResponse, PrimitiveError>;
```

The role is `plausibility`. Usually a small model (the decision is
binary; the rationale is the only place reasoning depth matters).

The judge's role is to prevent the adversary from winning by being
silly (per ADJ05): plausible iff a competent practitioner in the
domain would actually interpret the source that way. An
`IMPLAUSIBLE` verdict logs the adversary's reading but does not
fail the adjudication.

## extract_rules

The rule-compilation primitive for ADJ09.

```rust
pub struct ExtractRulesRequest {
    pub rulebook_segment:  String,
    pub segment_id:        String,           // for audit trail provenance
    pub rule_schema:       JsonSchema,       // ADJ01 v2 Rule node schema
    pub as_of:             ChronoDate,
    pub domain_hints:      DomainHints,
}

pub struct ExtractRulesResponse {
    pub rules:        Vec<IRNode>,           // all kind = Rule
    pub coverage:     SegmentCoverage,       // which parts of the segment produced rules
    pub call_record:  LlmCallRecord,
}

pub async fn extract_rules(
    req: ExtractRulesRequest,
    gateway: &GatewayConfig,
) -> Result<ExtractRulesResponse, PrimitiveError>;
```

The primitive returns one or more Rule IR nodes per segment. The
`SegmentCoverage` tracks which parts of the source segment produced
which rules — for the rule-compilation pipeline's structural-
coverage check (ADJ09 §"Coverage check").

The role is `extractor` (same as document extraction; the deployment
can configure separately if desired).

## Error type

```rust
pub enum PrimitiveError {
    /// Gateway-level error (network, auth, rate limit, etc.)
    Gateway(LlmError),

    /// The LLM's output failed schema validation after retries.
    ValidationExhausted {
        last_response: String,
        last_error:    String,
        attempts:      usize,
    },

    /// The output passed JSON schema but failed framework-specific
    /// well-formedness or coverage checks.
    StructuralFailure {
        check_name: String,
        detail:     String,
    },

    /// The deployment's GatewayConfig has no client for the required role.
    NoClientForRole { role: String },
}
```

Every variant carries enough information for ADJ06 to render a
clarification question or for telemetry to attribute the failure.

## Prompt Versioning

Each primitive has a `PromptVersion` constant:

```rust
pub const DECOMPOSE_TEXT_PROMPT_VERSION: &str = "decompose-text-v1";
pub const RENDER_NODE_PROMPT_VERSION:    &str = "render-node-v1";
pub const ENTAIL_PROMPT_VERSION:         &str = "entail-v1";
pub const ADVERSARY_PROMPT_VERSION:      &str = "adversary-v1";
pub const PLAUSIBILITY_PROMPT_VERSION:   &str = "plausibility-v1";
pub const EXTRACT_RULES_PROMPT_VERSION:  &str = "extract-rules-v1";
```

The version string appears in the `LlmCallRecord` for every call
the primitive makes. Bumping the version is the audited way to
change a prompt; replay matches on `(prompt_version, prompt_hash)`.

Prompt templates themselves live in `code/packages/rust/llm-primitives/
src/prompts/` and are loaded at startup. They are plain text files;
the framework does not invent its own template language.

## Caching

The primitive layer is the right place for a **response cache** —
not at the gateway level, where the cache key would need to capture
every request field, but at the primitive level, where the
semantically meaningful inputs are well-defined.

```rust
pub struct PrimitiveCache {
    pub backend:  Box<dyn CacheBackend>,   // in-memory / Redis / S3
    pub ttl:      Duration,
}
```

Cache keys are `(primitive_name, prompt_version, hash(input))`.
Cache hits skip the LLM call entirely; the audit trail records the
hit and the cached response's original `LlmCallRecord` (preserving
provenance to the original call).

Caching is opt-in per deployment. The default `decompose_text` is
**not cached** because the LLM call is the bulk of the work; but
`entail` over short premise/hypothesis pairs caches well, and
`render_node` over structurally identical leaves caches even better.

## Audit Trail Integration

Every primitive call produces an `LlmCallRecord` (from `LM00`). The
record is wrapped in a `PrimitiveCallRecord` that adds primitive-
specific context:

```text
PrimitiveCallRecord := {
    primitive:        string,         -- "decompose_text", "entail", ...
    prompt_version:   string,
    role:             string,
    inputs_hash:      Sha256Hex,
    outputs_hash:     Sha256Hex,
    cache_hit:        bool,
    attempts:         usize,          -- retry count
    llm_calls:        [LlmCallRecord], -- one per attempt
    total_cost_usd:   f64,
}
```

The PrimitiveCallRecord is emitted into the document's audit trail
via the standard ADJ07 schema.

## Composition with Clarification (ADJ06)

When a primitive fails (validation exhausted, structural failure),
the failure is surfaced to ADJ06's clarification dialogue:

| Primitive failure | ADJ06 clarification kind |
|---|---|
| `decompose_text` → ValidationExhausted | re-prompt the LLM with the validation error |
| `decompose_text` → StructuralFailure (coverage gap) | UncoveredSpan clarification, surface the gap to the user |
| `entail` → low confidence in both directions | RoundTripDrift clarification, ask for rephrase |
| `find_contradicting_reading` → plausible reading | AdversarialReading clarification, ask user to confirm |
| `extract_rules` → SegmentCoverage incomplete | Cite the unaccounted-for sub-segment to the rule reviewer |

The primitive doesn't *implement* ADJ06; it returns a typed failure
shape that ADJ06 consumes.

## Implementation Sketch

```text
code/packages/rust/llm-primitives/
   src/
     lib.rs                        -- the six functions
     prompt_version.rs             -- constants
     prompts/                      -- text templates
       decompose_text.md
       render_node.md
       entail.md
       adversary.md
       plausibility.md
       extract_rules.md
     schemas/                      -- JSON Schema for each primitive's output
       ir_document.schema.json
       rule_node.schema.json
       entail_response.schema.json
       ...
     cache.rs                      -- PrimitiveCache + in-memory backend
```

Tests use the mock provider extensively. Every primitive has a
test that supplies a scripted mock response and verifies the parsed
output. Integration tests use a small public Anthropic / OpenAI key
in CI to verify the real-provider paths work; these tests are gated
by an env var so contributors without keys can still run the bulk
of the suite.

## Open Questions

1. **Streaming for `decompose_text`.** Large documents produce
   large IR outputs; streaming the IR (one TextRun at a time) would
   improve responsiveness. Requires the schema to support partial
   trees and the primitive to surface progress. Open.
2. **Few-shot examples in prompts.** Currently the prompts include
   the schema but no examples. Adding curated examples (e.g., a
   TSA-style adjudication example) typically improves quality at
   the cost of token usage. Whether examples should be domain-
   conditional (different examples per `DomainHints`) is a
   deployment-experience question.
3. **Tool use vs. JSON output.** Some providers handle structured
   output more reliably via tool use than JSON mode. The primitive
   currently uses JSON mode uniformly; a tool-use path is a per-
   provider optimisation worth tracking.
4. **Re-prompt-with-correction loops.** The retry-with-correction
   protocol is currently a fixed N-attempts shape. A more
   sophisticated agent loop (with reasoning, partial responses,
   tool calls) is `LM04` (a planned spec) — but the primitive
   exposes the *outcome* to callers, who don't see the loop's
   internals.

## Limitations

1. **The primitives are pure functions, not agents.** They take an
   input, produce an output, return. They don't maintain state
   across calls or initiate multi-step interactions on their own.
   ADJ06 (clarification) is the agentic layer; primitives are its
   atoms.
2. **Schema bloat in prompts**. The IR schema is large (the full
   ADJ01 v2 grammar) and consumes tokens on every `decompose_text`
   call. Prompt caching mitigates but does not eliminate this. A
   deployment with very high call volume may want to compile a
   compact prompt variant for production while keeping the full
   schema for development.
3. **Domain hints are blunt**. `DomainHints::Clinical` is a single
   enum value but real clinical text varies enormously (emergency
   department, primary care, psychiatry, surgical). Refining hints
   into a richer taxonomy is deployment work.

## Status

Draft. Sufficient for the `llm-primitives` Rust crate to begin.
Depends on `LM00` (gateway trait) and `LM00a` (provider
implementations) for compilation; depends on `ADJ01 v2` for the IR
schema.
