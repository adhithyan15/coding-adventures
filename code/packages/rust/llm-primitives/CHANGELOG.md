# Changelog

All notable changes to this project will be documented in this file.

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
