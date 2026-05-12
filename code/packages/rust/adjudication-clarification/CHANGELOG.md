# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-05-12

### Added

Self-correction loop now covers **ADJ03 polarity/modality**
failures in addition to ADJ02 coverage. The qwen2.5:1.5b benchmark
in [ADJ12](../../specs/ADJ12-small-model-benchmarks.md) showed the
framework recovering from ADJ02 via clarification but then getting
stuck on ADJ03; v0.2 closes that gap.

- `PolarityClarificationRequest { original, violation_description,
  previous_ir, polarity_hint: Option<String> }` — the input shape.
- `retry_decompose_on_polarity_failure(req, gateway, max_attempts,
  now)` — headline entry point. Same retry-loop machinery as the
  coverage variant; the difference is the correction prompt.
- `POLARITY_CLARIFICATION_PROMPT_VERSION = "polarity-clarification-v1"`
  — separate from `CLARIFICATION_PROMPT_VERSION` so audit-trail
  replay can distinguish the two correction flavours.
- `build_polarity_correction_prompt` — emits a prompt that lists
  the legal polarity/modality values, calls out negations as the
  most common failure mode, and optionally embeds a framework hint
  about which node is wrong and why.
- Internal `retry_with_correction_prompt` helper shared between
  the coverage and polarity entry points; the retry loop is now
  defined once, not duplicated.

5 new tests cover the polarity path: happy-success, prompt with
hint, prompt without hint, prompt-version constant lock, and
graceful exhaustion on repeated errors.

### Notes

ADJ04 (round-trip drift) and ADJ05 (adversarial reading) still
need their own correction shapes — they're about renderer drift,
not IR extraction, so the fix is "re-render this node" not
"re-extract the IR." Those land in follow-ups.

## [0.1.0] - 2026-05-12

### Added

ADJ06 clarification dialogue scaffold. When a checker pass surfaces
a violation, this crate re-prompts the LLM with the structured
diagnostic and tries again.

- `retry_decompose_on_coverage_failure(req, gateway, max_attempts, now)` —
  the headline entry point. Re-runs `decompose_text` up to
  `max_attempts` times, each time prepending a correction prompt
  that includes the ADJ02 violation description and the model's
  previous IR JSON.
- `CoverageClarificationRequest` / `CoverageClarificationOutcome`
  shape the in/out contract.
- `ClarificationError::Exhausted` carries the full dialogue trail
  so callers can escalate (Rung 2 / Rung 3) with full context.
- `ClarificationError::Primitive` distinguishes "model produced bad
  output" from "the gateway is down" (transport / auth failure).
- `CLARIFICATION_PROMPT_VERSION = "clarification-v1"` records the
  prompt-template version in every `DialogueTurn`.
- 8 tests cover: first-success path, prompt-content (violation +
  previous IR), actor-id recording, prompt-version recording, the
  version constant lock, exhaustion path (records Abandoned
  outcomes), happy-on-second-attempt sanity, and zero-max-attempts
  is treated as one.

### Notes

- v0.1 keeps the loop simple: ask, receive, hand back to the caller.
  This crate does NOT re-validate coverage on the new IR — that's
  the pipeline's job. The pipeline re-runs ADJ02 and either accepts
  the corrected IR or loops back into this crate.
- Other violation types (ADJ03 polarity/modality, ADJ04 round-trip
  drift, ADJ05 adversarial readings) need their own correction
  shapes and land as follow-ups. v0.1 focuses on coverage because
  that's the most common small-model failure mode (cf. the TSA
  demo's `LlmExtracted` mode against `gemma4:latest`).
- Rung 1 only (same model re-prompt). Rung 2 (different model) and
  Rung 3 (human) follow.
