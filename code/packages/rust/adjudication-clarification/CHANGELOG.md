# Changelog

All notable changes to this project will be documented in this file.

## [0.4.0] - 2026-05-12

### Added

**The self-correction story is complete.** v0.4 adds ADJ06-for-ADJ05:
when the adversary finds a plausible alternative reading, the
framework re-prompts the extractor with the reading attached and
asks for a more precise IR (or an explicit `Uncertainty` node).
With v0.4, all four LLM-touched checker passes have their own
self-correction shape:

| Checker | Failure | Retry primitive | Outcome |
|---------|---------|-----------------|---------|
| ADJ02 coverage | byte not covered | `decompose_text` | corrected JSON IR |
| ADJ03 polarity | wrong polarity/modality | `decompose_text` | corrected JSON IR |
| ADJ04 round-trip | rendering drifts | `render_node` | corrected rendering string |
| ADJ05 adversarial | plausible alt reading | `decompose_text` | corrected JSON IR |

- `AdversarialClarificationRequest { original, previous_ir,
  adversary_reading, adversary_explanation, judge_reason }` —
  the input shape, carrying the adversary's reading + the judge's
  ruling.
- `retry_decompose_on_adversarial_failure(req, gateway, max_attempts, now)`
  — headline entry point. Reuses the shared retry-loop machinery.
- `ADVERSARIAL_CLARIFICATION_PROMPT_VERSION = "adversarial-clarification-v1"`
  for audit-trail replay.
- `build_adversarial_correction_prompt` offers the model **two
  fix paths**:
  1. **Be more specific** — refine the relevant Fact's term so the
     alternative becomes clearly wrong.
  2. **Mark the ambiguity** — add an `Uncertainty` node or flip a
     polarity to `Uncertain`.
- 4 new tests cover the adversarial path. Total 24 tests in the
  crate (8 coverage + 5 polarity + 7 render + 4 adversarial).

### Notes

This completes the self-correction matrix for the four fallible
LLM-touched checker passes. The framework can now respond to ANY
of the five gating/advisory failures with a structured retry
prompt that points the model at the specific defect.

Wiring these new retries (ADJ04 + ADJ05) into the demos' clarification
loops is a follow-up — v0.4 ships the primitive surface only.

## [0.3.0] - 2026-05-12

### Added

Self-correction loop now covers **ADJ04 round-trip drift** failures.
When `adjudication-round-trip::check_round_trip` flags a node
because the rendering drifts from source, the framework can
re-prompt the renderer (a different primitive than decompose_text)
with the drift direction and let it self-correct.

- `RenderClarificationRequest { original, previous_rendering,
  failing_direction: Option<DriftDirection>, drift_description }`
  — the input shape.
- `RenderClarificationOutcome { corrected_rendering, dialogue,
  used_attempts }` — distinct from `CoverageClarificationOutcome`
  because the corrected output is a String, not a JSON IR.
- `retry_render_on_drift_failure(req, gateway, max_attempts, now)`
  — the headline entry point. Same retry-loop spirit as
  coverage/polarity, but uses `render_node` instead of
  `decompose_text` because the unit of correction is a single
  node's rendering, not the whole IR.
- `DriftDirection { SourceToRendering, RenderingToSource, Both }`
  + `DriftDirection::classify(p_to_h, h_to_p, threshold)` —
  resolves the failure mode from the two NLI scores so the
  correction prompt can focus.
- `build_render_correction_prompt` — emits a focused prompt:
  - SourceToRendering: "you added or fabricated content; trim it"
  - RenderingToSource: "you dropped detail; add it back"
  - Both: "drift in both directions"
  - With explicit advice to render ambiguity as ambiguity rather
    than guess.
- `RENDER_CLARIFICATION_PROMPT_VERSION = "render-clarification-v1"`
  — distinct from the coverage/polarity versions so audit-trail
  replay can tell the three correction flavours apart.

7 new tests cover the render-drift path: happy-success,
direction-aware prompt content, both-directions handling,
no-direction (generic prompt), `DriftDirection::classify`
truth table, prompt-version constant lock, and graceful
exhaustion on repeated renderer errors.

### Notes

ADJ05 (adversarial reading) still needs its own correction shape —
it's about the adversary finding a plausible alternative reading,
not about renderer drift. Follows as a separate PR.

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
