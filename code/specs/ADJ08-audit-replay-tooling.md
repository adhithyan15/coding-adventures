# ADJ08 — Audit Replay Tooling: Re-running an Adjudication from Its Trail

## Overview

[`ADJ07`](ADJ07-audit-trail-schema.md) defines the audit trail's
shape. This spec defines the **replay tool** that takes a stored
audit trail and re-executes the adjudication, comparing the replay's
output against the trail's recorded artifacts.

Replay matters for three reasons:

1. **Compliance.** Regulators and internal auditors must be able to
   verify a past adjudication was reached correctly. Replay produces
   the verification.
2. **Regression detection.** When a model is upgraded, a checker pass
   refined, or a rule revised, replaying historical adjudications
   surfaces behavior changes — both desired and undesired.
3. **Reproducibility.** Research papers and clinical case reviews
   require reproducibility. The replay tool plus the audit trail plus
   the configuration record together make a closed-loop reproducible
   system.

## Layer Position

```
   ADJ07 audit trail schema   ← what is stored
        │
        ▼
   ADJ08 replay tooling        ← this document; consumes a trail
        │
   ┌────┴────┬────────┬────────┐
   ▼         ▼        ▼        ▼
   ADJ02..ADJ05  ADJ06     LP19 engine
   (re-run)    (replay)   (re-run)
```

## What the Replay Does

Given a complete `AuditTrail`, the replay tool:

1. **Reconstructs the documents** from the trail's `documents` field.
   Normalized text and the original byte sequence are recovered.
2. **Re-runs extraction** on each document using the configuration's
   `extractor_model` and `extractor_prompt`. Output is compared
   structurally against the trail's `ir_nodes` field.
3. **Re-runs every checker pass** using the configurations's
   versioned components. Outputs compared against `checker_results`.
4. **Re-executes the dialogue** turns where the response source is
   `Extractor` (deterministic re-prompts), `EHR` (deterministic
   re-queries), or `Imputed`. User and Expert turns are *replayed
   from the recorded responses* — replay does not re-prompt humans.
5. **Re-runs the engine** with the same KB and search mode.
6. **Computes per-artifact equality** between replay output and the
   stored trail.

The result is a `ReplayReport`:

```text
ReplayReport := {
    trail_id:          AdjudicationId,
    replay_started_at: ISO-8601,
    replay_completed:  ISO-8601,
    outcome_equal:     bool,
    artifact_diffs:    [ArtifactDiff],
    engine_diff:       Option<EngineDiff>,
    overall_verdict:   Match | Mismatch(reason) | UnreplayableInputs(reason),
}

ArtifactDiff := {
    pass_or_node: string,    -- e.g., "ir_nodes[F3]" or "ADJ02_coverage"
    expected:     Json,
    actual:       Json,
    kind:         FieldMismatch | StructuralDifference | OrderingDifference,
    severity:     Critical | Minor,
}
```

## What Counts as "The Same"

Replay equality is delicate. Three classes of comparison:

### Structural Equality (Critical)

Two artifacts are structurally equal iff they have the same shape and
the same content at every position. The trail's `ir_nodes` and the
replay's `ir_nodes` must satisfy this for the replay to be considered
a match.

Configurable: `confidence` fields may be allowed to differ by a small
tolerance (LLMs are not perfectly deterministic in their reported
confidence), but `id`, `kind`, `term`, `polarity`, `modality`,
`source_spans`, `lowered_from`, `discard_reason`, and `metadata` must
match exactly.

### Semantic Equality (Minor)

Some differences are *semantic* — same meaning, different surface
form. Examples:

- Term `[a, b, c]` vs. `'.'(a, '.'(b, '.'(c, [])))` — equivalent under
  the canonical encoding.
- Round-trip rendering text — the rendering may not be byte-equal
  across runs of the same LLM version (temperature, sampling). The
  *NLI score* should match; the literal text may not.
- Dialogue response timestamp — non-deterministic; ignored.

The replay reports semantic differences as `Minor` and continues. A
configuration knob (`strict-replay = true`) escalates semantic
differences to Critical.

### Provenance Equality (Critical)

Every IR node's `source_spans` must reference the same document byte
ranges. Every proof step's `via_facts` / `via_rules` must point to
identifying fact/rule ids. These are the provenance properties — they
must replay byte-equal.

## Configuration Drift

The replay tool detects three kinds of configuration drift:

1. **Component version drift.** The trail records each model and tagger
   with a version stamp. If the replay configuration uses different
   versions, the tool reports this as `UnreplayableInputs` and either
   fails fast (default) or proceeds with the new versions and reports
   any resulting artifact diffs (with `--allow-drift`).
2. **Prompt drift.** The extractor's prompt has changed between trail
   time and replay time. Same handling.
3. **Rule corpus drift.** A rulebook used in the original adjudication
   has been updated. The trail's `as_of` field selects the historically
   correct rule version; if the replayer's KB does not have that
   version available, fail.

The point: **a replay must be explicit about what changed.** Silent
drift is the failure mode.

## Replay Modes

```text
ReplayMode :=
    Strict           -- structural equality required; fail on minor diffs
  | Standard          -- structural equality on critical fields; minor diffs
                        reported but accepted
  | AllowDriftReport  -- replay with current configuration even if it
                        differs from the trail's; report all diffs

ReplayScope :=
    Full             -- re-run extraction, checkers, dialogue, engine
  | EngineOnly        -- skip extraction; use the trail's IR; re-run
                        only the engine. Useful for engine regressions.
  | CheckersOnly      -- skip extraction and engine; re-run the four
                        checker passes against the trail's IR. Useful
                        for checker refinement.
```

`EngineOnly` is the fastest and most determinic replay (no LLM calls).
`Full` is the most thorough but slowest and most non-deterministic.

## CLI Sketch

```text
adj-replay --trail audit.json --mode strict --scope full
    -> ReplayReport printed; exit code 0 if match, 1 if mismatch.

adj-replay --trail audit.json --mode allow-drift --scope engine-only
    -> Useful for: "Did my engine change behavior on this case?"

adj-replay --trail audit.json --report-only
    -> No replay; just lints the trail's internal consistency
       (every span points into a real document range; every proof
       step cites an existing clause; etc.).
```

## Internal Trail Consistency Check

A useful subcomponent of the replay tool is the **trail consistency
linter**: it does not re-run anything, but verifies that the trail's
own internal references are consistent. Conditions checked:

- Every `Span.document_id` is in `documents`.
- Every `Span` is a valid byte range in its document's normalized
  text.
- Every IR node's `lowered_from` points to a node in the same trail.
- Every dialogue turn's `failure.node_id` exists.
- Every engine artifact's `via_facts` / `via_rules` reference clauses
  declared in the trail's KB.
- The lowering DAG is acyclic.

The linter runs in milliseconds and is suitable for use as a
pre-storage validation step.

## Worked Example

Take the audit trail produced by `ADJ10`'s TSA example. Run replay:

```text
$ adj-replay --trail tsa-2026-05-11.json --mode strict --scope full
Loading trail tsa-2026-05-11.json...
Configuration:
  extractor_model:  anthropic/claude-opus-4-7@2026-05-10
  renderer_model:   anthropic/claude-haiku-4-5@2026-05-10
  nli_model:        deberta-v3-base-mnli@2024-09-15
  ... (all match current deployment)

Replaying extraction on tsa-2026-05-11-001...
  IR nodes: 8 expected, 8 produced. structurally equal.

Replaying ADJ02 coverage...    PASS, matches trail.
Replaying ADJ03 polarity...     PASS, matches trail.
Replaying ADJ04 round-trip...   PASS (1 missing-input flag on F3),
                                 matches trail.
Replaying ADJ05 adversarial...  PASS, matches trail.

Replaying dialogue:
  Turn 1 (Rung0): re-prompt failed; matches trail.
  Turn 2 (Rung2): User response replayed; new spans appended;
                  matches trail.

Re-running engine:
  Search mode: FindFirst (KB is all-Certain). matches trail.
  Per-item verdicts: all match.

VERDICT: Match
```

If the renderer model is upgraded between adjudication and replay, the
`Standard` mode tolerates a minor diff in the rendered text as long
as the NLI scores match. `Strict` mode would fail this.

## Differential Replays

A particularly useful capability: replay the same trail under *two*
configurations and compare. The most common use case is *"is the new
extractor model better or worse on this case?"*:

```text
adj-replay --trail audit.json --mode allow-drift --baseline-config old-config.yaml --current-config new-config.yaml --diff
```

The tool runs the replay twice — once with the old configuration,
once with the new — and produces a per-artifact diff showing where
the new system disagrees with the old.

This is the deployment-time tool for evaluating extractor upgrades.

## Storage and Retention

Replay tooling is operationally cheap (per replay, no human time;
LLM calls cost the inference, ~$0.10–$1 per medical adjudication
replay at current prices). The constraint is **storage**:

- A clinical audit trail can be 100KB–1MB.
- Hospitals adjudicating 10,000 cases/week produce ~1–10 GB/week.
- Replay over a year's trails for evaluation purposes is a real cost.

The framework recommends two retention modes:

1. **Full retention** — every trail kept indefinitely. Suitable for
   high-stakes domains with regulatory mandates (medical malpractice
   tail: 7–10 years).
2. **Sample retention** — keep every trail for 30–90 days, then sample
   a fixed fraction (e.g., 10% stratified by outcome) for long-term
   retention. The sampled set supports regression evaluation.

Sample retention is a deployment policy decision, not a framework
concern. The framework provides hooks (a trail's `retention_class` is
metadata) but does not enforce policy.

## Open Questions

1. **Non-deterministic LLM outputs.** Even at temperature 0, LLM
   outputs are not perfectly reproducible (numerical precision,
   floating-point ordering, batching effects). The replay tool's
   tolerance for these is *configurable* but the right default is
   empirical. `ADJ08a` covers calibration.
2. **Partial trails.** A trail terminated by `ClarificationExhausted`
   is replayable up to the termination point but not beyond. The
   replay reports what was reproducible vs. unreachable.
3. **Privacy in replay.** If trails contain PHI, replay over a
   regression-eval set must use the same privacy controls as
   production. Out of scope here.
4. **Differential replays across rulebook versions.** "Would this case
   have come out differently under last year's rules?" Supported by
   the `as_of` mechanism; the UX of presenting the comparison is
   deployment-specific.

## Limitations

1. **Replay cannot recover from a corrupt trail.** Trail consistency
   linting helps detect corruption, not repair.
2. **User responses cannot be re-elicited.** A Rung2 or Rung3 dialogue
   turn's response is recorded once; replay uses the recording. If the
   replay needs a different response (e.g., to test a counterfactual),
   that is a *new* adjudication, not a replay.
3. **Network-dependent integrations** (e.g., EHR queries on Rung1) may
   no longer work at replay time. The trail records the response
   text from the original query; replay uses the recorded text. If
   the recorded text is unavailable, replay reports
   `UnreplayableInputs`.

## Status

Draft. Sufficient to implement directly. `ADJ08a` (LLM non-determinism
tolerance calibration) is a planned follow-up tied to deployment
experience.
