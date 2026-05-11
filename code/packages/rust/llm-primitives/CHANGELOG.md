# Changelog

All notable changes to this project will be documented in this file.

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
