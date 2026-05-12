# Changelog

All notable changes to this project will be documented in this file.

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
