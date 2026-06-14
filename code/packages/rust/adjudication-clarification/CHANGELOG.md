# Changelog

All notable changes to this project will be documented in this file.

## [0.7.0] - 2026-05-13 — retry_decompose_level routes through decompose_level (content-shaped)

### Contract change: retry prompt shows missing text, not byte ranges

`build_hierarchical_correction_prompt` no longer asks the model to
reason about byte offsets. Gap descriptions from the orchestrator
are already content-shaped (literal missing substrings); the retry
prompt embeds them verbatim and asks the model to "redo the
decomposition" with a `text` field per child. Per
`feedback_no_byte_arithmetic_for_llm`.

Otherwise, the v0.7.0 entry below applies.



### Changed

`retry_decompose_level` now calls `llm_primitives::decompose_level`
under the hood instead of `decompose_text`. The level-aware system
prompt (per `DecomposeLevel`) replaces the v5 flat-IR system prompt
that was the load-bearing cause of the ADJ26 bench's 0/40 result.

The retry primitive's correction-prompt logic stays intact: the
prior attempt + gap description are rendered into a
correction-context string and flow into `decompose_level` as
`correction_context`. On initial-dispatch paths where the
orchestrator funnels through this primitive with an empty
`previous_children` array, the correction context is suppressed —
the model sees the level-aware system prompt + parent text alone,
without a confusing "your previous attempt was empty" framing.

### New behaviour

- Per-level dispatches see one focused system prompt instead of the
  v5 monolith.
- Initial dispatches with empty prior children skip the correction
  context (the new prior-is-empty detector reads
  `previous_children.nodes` and checks if the array is empty).
- The `prompt_version` recorded in the audit-trail dialogue turn is
  unchanged (`HIERARCHICAL_DECOMP_PROMPT_VERSION`) since the
  primitive's own version is captured in its `call_record`.

### Tests

All 43 existing tests pass unchanged. The retry primitive's surface
is unchanged — only the underlying LLM call changed.

### Notes

- Version: 0.6.0 → 0.7.0 (additive behaviour shift; surface
  unchanged).

## [0.6.0] - 2026-05-13 — ADJ25 PR-3: fresh-agent hierarchical-decomposition retry

### Added

New primitive `retry_decompose_level(req, gateway, max_attempts,
now)` per [ADJ25](../../specs/ADJ25-hierarchical-decomposition.md).
The hierarchical retry is **fresh-agent per attempt** and
**parent-scoped** — fundamentally different from the existing
whole-document retries (coverage / polarity / drift / adversarial):

- Each attempt is a stateless LLM call. The prompt is built fresh
  from the framework's state; no conversation history flows between
  attempts.
- The model sees the parent's text (e.g., 14 bytes of "1 carry-on
  bag"), the previous attempt's children, and a plain-English
  description of the gap. Not the whole document.
- The prompt is **source-shaped, not framework-shaped**. No
  references to ADJ## numbers, "decomposition invariants", or other
  framework jargon. The model is asked to look at a chunk of text
  and produce a complete decomposition.

### New public surface

- `DecompositionLevel` — four variants matching
  `adjudication_coverage::DecompLevel` 1:1. Defined locally to
  avoid taking a dependency on the coverage crate; the orchestrator
  (PR-4) bridges the two.
- `HierarchicalDecompRetryRequest` — `{ level, document_id,
  parent_text, previous_children, gap_description, ancestor_context }`.
- `HierarchicalDecompRetryOutcome` — `{ corrected_children,
  dialogue, used_attempts }`. The `corrected_children` field
  carries the parent's NEW children JSON (not a whole document).
- `HIERARCHICAL_DECOMP_PROMPT_VERSION = "hierarchical-decomp-v1"`
  — stable prompt-version constant for audit-trail replay.

### Scope and what PR-3 deliberately does not do

- **No coverage re-verification.** The primitive hands back whatever
  JSON the model produced; the orchestrator (PR-4) re-runs
  `check_hierarchical_coverage` and either accepts or calls back in
  for another retry. Decoupling keeps this crate independent of
  `adjudication-coverage`.
- **No orchestration.** A single retry call addresses a single gap.
  PR-4 drives the full level-by-level decomposition loop, picking
  which parent/gap to retry on each iteration.

### Tests

8 new test cases covering: happy-path retry, prompt-language is
source-shaped not framework-shaped, per-level wording, ancestor
context rendered when provided, control-character sanitization,
1-attempt budget, prompt-version constant lock, level noun helpers.
Total `adjudication-clarification` tests: 32 → 40, all passing.

### Notes

- A top-level `sanitize_for_prompt(s, max_len)` helper was added.
  It coexists with the local-function `sanitize_for_prompt` inside
  `build_typed_quantity_correction_prompt` — Rust's name-resolution
  scopes the local one to its enclosing fn, no clash.
- Version: 0.5.0 → 0.6.0 (additive public surface).

## [0.5.0] - 2026-05-13 — ADJ06-for-ADJ22 typed-quantity retry

### Added

`retry_decompose_on_typed_quantity_failure(req, gateway, max_attempts, now)`
— the typed-quantity retry primitive (ADJ06-for-ADJ22).

When [ADJ22](../../../specs/ADJ22-typed-quantity-coverage.md)
finds a numerical literal in the source that the model failed to
wrap in a `quantity(value, unit)` compound, this function
re-prompts the same extractor with **pinpoint feedback** — the
specific literal value, its byte range, and which existing IR
nodes it overlaps. Empirically (per
[ADJ23](../../../specs/ADJ23-decomposition-bench.md))
this is meaningfully better than re-running the v5 system prompt
unchanged, because the model gets one targeted hint rather than
a generic "add typed quantities" reminder it already saw.

The function reuses `retry_with_correction_prompt` (the shared
inner loop already used by the coverage and polarity retries) so
the retry budget semantics, dialogue-turn shape, and exhaustion
behaviour stay aligned with the existing retries.

### New public types

- `MissingLiteralHint { literal, source_byte_range, nearby_node_ids }`
  — one per source literal the IR dropped. Constructed by the
  pipeline from `TypedQuantityViolation::MissingQuantity` records.
- `TypedQuantityClarificationRequest { original, violation_description,
  previous_ir, missing_literals }` — mirrors
  `CoverageClarificationRequest` shape plus the missing-literal list.

### New version constant

`TYPED_QUANTITY_CLARIFICATION_PROMPT_VERSION = "typed-quantity-clarification-v1"`
— distinct from the coverage / polarity / drift / adversarial
versions so audit-trail replay can tell the typed-quantity
correction from the other flavours.

### Domain neutrality

`build_typed_quantity_correction_prompt` deliberately uses
domain-neutral examples (count, length, volume, mass, electrical,
temperature, clinical, fractions). A regression-guard test
(`typed_quantity_prompt_is_domain_neutral`) asserts the prompt
contains none of `tsa`, `screening officer`, `passenger`,
`doctor`, `patient`, `clinician`, `lawyer`, `contract attorney`
— so future edits don't drift back toward domain bias.

### Defense in depth: prompt-input sanitization

`build_typed_quantity_correction_prompt` sanitizes both `literal`
and `nearby_node_ids` before embedding them into the retry
prompt: control characters are stripped, backticks are removed
(can't escape a surrounding markdown code-fence), and overlong
strings are truncated (literals → 32 chars, node IDs → 64 chars).
These values originate from LLM-emitted IR (node IDs) and source
text (literals); the sanitization prevents a malicious or
buggy extractor from injecting newline-prefixed pseudo-system
instructions into the prompt that's sent back to the same model
on the retry.

### Tests added (8)

- `typed_quantity_retry_returns_corrected_ir_on_first_success`
- `typed_quantity_correction_prompt_handles_empty_missing_list`
- `typed_quantity_correction_prompt_attaches_new_node_when_no_overlap`
- `typed_quantity_clarification_prompt_version_is_locked`
- `typed_quantity_prompt_is_domain_neutral`
- `typed_quantity_prompt_sanitizes_control_characters_in_literal_and_node_ids`
- `typed_quantity_prompt_truncates_overlong_literal_and_node_ids`
- `typed_quantity_retry_exhausts_gracefully_on_repeated_error`

Self-correction story now covers ALL five LLM-touched checker
passes: ADJ02 coverage, ADJ03 polarity/modality, ADJ04 round-trip
drift, ADJ05 adversarial reading, ADJ22 typed-quantity coverage.

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
