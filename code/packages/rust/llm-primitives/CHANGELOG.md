# Changelog

All notable changes to this project will be documented in this file.

## [0.6.0] - 2026-05-11

### Added

- `decompose_text` module — **the headline extraction primitive**.
  Given source text + a domain hint, calls the `Extractor` role's
  client to produce a hierarchical IR document (per ADJ01 v2). The
  pipeline runs `check_coverage` and `check_propagation` against the
  result.
- `DecomposeTextRequest { document_id, source_text, domain_hint,
  language_hint }` and `DecomposeTextResponse { ir_document,
  structural_ok, call_record }`.
- `ir_document` is a `serde_json::Value` at v0.6 because
  `adjudication_ir::IRDocument` doesn't yet derive `Serialize` /
  `Deserialize`. A future version will swap to the typed shape; the
  on-wire JSON is unchanged.
- Lightweight structural sanity check: `structural_ok = true` iff
  the response is an object with a non-empty string `document_id`
  matching the request AND an array `nodes`. Full ADJ01 v2
  well-formedness lives in `adjudication_ir::validate` and is the
  caller's job.
- Routes via `LlmClient::complete_json` with `max_tokens: 8192`
  (IR documents for long sources can run to several thousand
  tokens). Schema: top-level object with required `document_id`
  (string ≥ 1 char) and `nodes` (array), `additionalProperties:
  true` so the LLM can include richer per-node fields beyond the
  primitive's minimal probe.
- `LlmCallRecord`: `primitive="decompose_text"`, `role="extractor"`,
  `prompt_version="decompose-text-v1"`, content-addressed
  `prompt_hash`, provider, usage, latency.
- Re-exported at the crate root: `decompose_text`,
  `DecomposeTextRequest`, `DecomposeTextResponse`.
- 11 new tests. Coverage: `NoClientForRole` when Extractor
  unregistered; happy path with full IR + call record; user message
  tags DOMAIN / LANGUAGE / DOCUMENT_ID / SOURCE; missing
  `language_hint` renders `"auto-detect"`; `ContextTooLarge` gateway
  error propagates; non-object response → `ValidationExhausted`;
  `structural_ok` false-positives covered (missing / wrong
  `document_id`; non-array `nodes`); empty-`nodes` array is
  structurally OK; `prompt_hash` matches an independently-built
  request; the IR JSON round-trips unchanged.

### Notes

This is the **single-shot bottom layer** of the LM00b spec's retry-
with-correction loop. A future retry harness will wrap it with a
policy + count; the primitive owns the single LLM round-trip.

`domain_hint` and `language_hint` stay free-form strings at v0.6 to
avoid binding to a not-yet-existent `DomainHints` enum.

With this primitive, the **input side of the semantic source map is
complete** — source text → IR happens through the primitive layer.

## [0.5.0] - 2026-05-11

### Added

- `find_contradicting_reading` module — fourth concrete primitive from
  LM00b. ADJ05 **adversary**. Given a source span + the IR's
  rendering, find the strongest reading of the source that contradicts
  the IR — or report CONCURS if no plausible alternative exists.
- `FindContradictingReadingRequest { source_span_text, ir_rendered,
  domain_hint }` and
  `FindContradictingReadingResponse::Concurs { call_record } |
  Reading { text, explanation, call_record }`. Plus
  `FindContradictingReadingResponse::call_record()` accessor so
  callers can log without pattern-matching first.
- Re-exported at the crate root: `llm_primitives::find_contradicting_reading`,
  `FindContradictingReadingRequest`, `FindContradictingReadingResponse`.
- **Asymmetric** system prompt: "assume the extraction is wrong, find
  a reading that contradicts it" — per ADJ05, the asymmetry is the
  whole point.
- Routes via `LlmClient::complete_json` with a strict 3-field schema
  (`concurs: bool`, `text: string<=1024`, `explanation: string<=1024`,
  `additionalProperties: false`).
- Self-consistency validation: a model that returns `concurs: false`
  with empty `text` or empty `explanation` surfaces as
  `PrimitiveError::ValidationExhausted` rather than being silently
  treated as Concurs. ADJ06 sees the malformed response.
- `LlmCallRecord` populated: `primitive = "find_contradicting_reading"`,
  `role = "adversary"`, `prompt_version = "adversary-v1"`,
  content-addressed `prompt_hash`, provider, usage, latency.
- 11 new tests. Coverage: missing client → `NoClientForRole`; CONCURS
  response is recognised; full Reading response with text +
  explanation; user message tags SOURCE / IR-RENDERED / DOMAIN
  separately; gateway `Refused` propagates as `Gateway`; missing or
  wrong-typed `concurs` → `ValidationExhausted`; `concurs: false`
  with empty `text` or empty `explanation` → `ValidationExhausted`;
  Reading text and explanation are trimmed on success; `prompt_hash`
  matches an independently-built request.

### Notes

`domain_hint` is a string at v0.5 — the LM00b spec defines a
`DomainHints` enum; a follow-up will swap to the enum once that type
lands. Independence between `Role::Adversary` and `Role::Extractor`
is enforced at deployment time via `GatewayConfig::check_independence`,
not by the primitive.

With this primitive, the **ADJ05 adversarial triad is complete**:
`render_node` (v0.3) + `find_contradicting_reading` (v0.5) +
`judge_plausibility` (v0.4) — ADJ05's checker crate (a follow-up)
will compose them.

## [0.4.0] - 2026-05-11

### Added

- `judge_plausibility` module — third concrete primitive from LM00b.
  Binary judge for ADJ05's adversarial verifier. Takes
  `JudgePlausibilityRequest { source_span_text, ir_rendered,
  adversary_reading, domain_hint }`; returns
  `JudgePlausibilityResponse { plausible, reason, call_record }`.
  Decides whether a competent practitioner in the named domain
  would actually adopt the adversary's reading. An `IMPLAUSIBLE`
  verdict logs the adversary's reading in the audit trail but does
  not fail the adjudication; a `PLAUSIBLE` verdict surfaces as an
  `AdversarialReading` violation for ADJ06.
- Re-exported at the crate root: `llm_primitives::judge_plausibility`,
  `llm_primitives::JudgePlausibilityRequest`,
  `llm_primitives::JudgePlausibilityResponse`.
- Routes via `LlmClient::complete_json` with a strict 2-field schema
  (`plausible: bool`, `reason: string`, `additionalProperties: false`,
  `reason` `minLength: 1`, `maxLength: 1024`).
- `LlmCallRecord` populated: `primitive="judge_plausibility"`,
  `role="plausibility"`, `prompt_version="plausibility-v1"`,
  content-addressed `prompt_hash`, provider, usage, latency.
- 10 new tests. Coverage: `NoClientForRole` when plausibility role
  unregistered; happy path for both `plausible: true` and `false`;
  user message tags SOURCE / IR-RENDERED / ADVERSARY / DOMAIN
  separately; gateway `Auth` error propagates; missing `plausible`
  field → `ValidationExhausted`; wrong-typed `plausible` (string
  `"maybe"`) → `ValidationExhausted`; empty/whitespace-only `reason`
  → `ValidationExhausted`; reason is trimmed on success;
  `prompt_hash` matches an independently-built request.

### Notes

`JudgePlausibilityRequest.domain_hint` is a free-form string at v0.4.
The LM00b spec defines a `DomainHints` enum (None / Clinical / Legal /
TsaDeclaration / LicenseCompatibility / Custom); a follow-up will
swap to the enum once that type lands in a shared crate. Prompt and
wire shape unchanged across the upgrade.

## [0.3.0] - 2026-05-11

### Added

- `render_node` module — second concrete primitive from LM00b.
  Faithful natural-language rendering of one IR node. Takes a
  caller-formatted `node_description`, a `document_excerpt` for
  grounding, and a target `RenderStyle` (`Plain` / `Clinical` /
  `Legal`); returns a `rendering` string plus the `LlmCallRecord`.
- `RenderStyle` enum + `as_str()` for audit-trail tags and
  per-style prompt directives (plain English / clinical shorthand /
  formal legal register).
- Routes via `LlmClient::complete` (free-form text, not JSON).
  The only structural failure surfaced is whitespace-only output,
  which returns `PrimitiveError::ValidationExhausted` so ADJ06 can
  clarify. Substantive faithfulness is ADJ04's job via `entail`.
- `LlmCallRecord` populated with `primitive = "render_node"`,
  `role = "renderer"`, `prompt_version = "render-node-v1"`, content-
  addressed `prompt_hash`, plus provider identity, token usage,
  finish reason, and latency from the gateway response.
- 8 new tests. Coverage: `RenderStyle::as_str` stability,
  `NoClientForRole` when renderer is unregistered, happy-path
  trimmed-rendering with full `call_record` population, style
  directives in the user message (Clinical and Legal),
  whitespace-only response → `ValidationExhausted`, `RateLimit`
  propagates as `Gateway`, `prompt_hash` matches an
  independently-built request, `finish_reason` passes through to
  the call record.

### Notes

`render_node`'s `RenderNodeRequest.node_description` is a string at
v0.3 — the caller formats their IR node however they like. A
follow-up will swap to typed `adjudication_ir::IRNode` once that
crate ships its `serde` feature; the prompt and wire shape stay
unchanged.

## [0.2.0] - 2026-05-11

### Added

- `entail` module — first concrete primitive from LM00b. Bidirectional
  textual entailment: takes `EntailRequest { premise, hypothesis }`,
  returns `EntailResponse { premise_entails_hypothesis, p_to_h_score,
  hypothesis_entails_premise, h_to_p_score, call_record }`. Synchronous,
  matches the v0.1 `LlmClient` trait.
- Re-exported at the crate root: `llm_primitives::entail(...)`,
  `llm_primitives::EntailRequest`, `llm_primitives::EntailResponse`
  (function and module share the name; Rust allows this since they're
  in different namespaces).
- Stable system prompt for the `Role::Nli` slot baked into the module.
  Bumping `ENTAIL_PROMPT_VERSION` is the audited way to change it.
- Response JSON-schema validation:
  - Routes via `LlmClient::complete_json` so providers with native
    JSON mode use it.
  - Per-field check (booleans + scores) — a missing field returns
    `PrimitiveError::ValidationExhausted` with the raw response so
    ADJ06 can clarify.
  - Score range-check `[0, 1]`. Out-of-range or wrong-typed values
    surface as `ValidationExhausted`, not silent clamping.
- `LlmCallRecord` populated automatically: `primitive = "entail"`,
  `role = "nli"`, `prompt_version = "entail-v1"`, content-addressed
  `prompt_hash`, plus provider identity, token usage, and latency from
  the gateway response.
- 10 new tests covering: missing `Nli` client returns `NoClientForRole`;
  happy path round-trips; user message has `PREMISE:` / `HYPOTHESIS:`
  markers; gateway transport error propagates; missing / wrong-type /
  out-of-range fields all surface as `ValidationExhausted`; boundary
  scores (0.0 and 1.0) accepted; `call_record.prompt_hash` matches an
  independently-computed hash of the built request.

### Notes

`serde_json = "1"` is now a direct dep (used internally for JSON
parsing). The five remaining primitives (`decompose_text`,
`render_node`, `find_contradicting_reading`, `judge_plausibility`,
`extract_rules`) ship in follow-up PRs that can land in parallel —
each in its own module under `src/`.

## [0.1.0] - 2026-05-11

### Added

- `Role` enum: `Extractor`, `Renderer`, `Nli`, `Adversary`,
  `Plausibility`, `RuleExtractor`. Stable `as_str()` for audit-trail
  records.
- `GatewayConfig` — role-keyed registry of `Box<dyn LlmClient>` with
  builder-style `with_client`. Lookup via `client(Role)` returns
  `Option<&dyn LlmClient>`.
- `GatewayConfig::check_independence()` — ADJ05 startup check that
  `Role::Extractor` and `Role::Adversary` come from different
  `(vendor, model_family)` pairs. Returns `IndependenceViolation`
  with full provider identities on failure.
- `LlmCallRecord` — one row of the LLM audit trail. Carries
  `primitive`, `role`, `prompt_version`, `prompt_hash`, `provider`,
  `usage`, `finish_reason`, `latency_ms`, `cost_usd`. `PartialEq`
  only (not `Eq`) because `cost_usd: f64`.
- `PrimitiveCallRecord` — wraps one or more `LlmCallRecord`s (one
  per retry attempt) with primitive-level context (`inputs_hash`,
  `outputs_hash`, `cache_hit`, `attempts`, `total_cost_usd`).
- `PrimitiveError`: `Gateway(LlmError)`, `ValidationExhausted`,
  `StructuralFailure`, `NoClientForRole`. `From<LlmError>` impl
  for `?`-friendly propagation.
- Six prompt-version constants (`DECOMPOSE_TEXT_PROMPT_VERSION` …
  `EXTRACT_RULES_PROMPT_VERSION`) covering the LM00b primitive set.
- `fingerprint_prompt(&CompletionRequest) -> String` — deterministic
  FNV-1a-based hash of the prompt portion of a request, ignoring
  `temperature` and `seed` so retries match.
- 15 tests covering: `Role::as_str` stability, `GatewayConfig`
  registration / lookup, ADJ05 independence-check pass and fail
  cases (different families, same family same vendor, same vendor
  different families), `fingerprint_prompt` determinism and
  insensitivity to temperature/seed, `PrimitiveError` display, and
  prompt-version constant stability.

### Notes

This is the **skeleton-only** v0.1.0. The six primitive functions
(`decompose_text`, `render_node`, `entail`,
`find_contradicting_reading`, `judge_plausibility`,
`extract_rules`) ship in follow-up PRs that can land in parallel
because they don't conflict on a shared file.

Reference: [`LM00b`](../../../specs/LM00b-llm-primitives.md).
