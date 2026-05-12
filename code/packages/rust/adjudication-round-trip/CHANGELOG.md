# Changelog

All notable changes to this project will be documented in this file.

## [0.1.1] - 2026-05-11

### Fixed

Skip synthesized `Query` nodes (kind=Query AND
`source_spans.is_empty()`) instead of raising
`CheckError::LeafMissingSpans`. The previous behaviour broke any
pipeline that uses a programmatically-added Query node — common for
"what's the verdict?" style runs and for the ADJ10 TSA fixture. Facts
with missing spans still surface as `LeafMissingSpans` because Facts
must come from the source per ADJ01 v2 well-formedness.

One new test covers the Query-with-no-spans skip behaviour.

## [0.1.0] - 2026-05-11

### Added

ADJ04 round-trip checker. Composes the merged `render_node` and
`entail` primitives from `llm-primitives`.

- `check_round_trip(document_text, ir_doc, gateway, opts) -> Result<RoundTripResult, CheckError>`.
- `CheckOptions { threshold: f32, style: RenderStyle }` with sane
  defaults (0.6, Plain).
- `RoundTripResult { violations, call_records }` plus `pass()` helper.
- `RoundTripViolation { node_id, rendering, source_excerpt,
  source_to_rendering, rendering_to_source, threshold }`.
- `CheckError::Primitive(PrimitiveError)` / `LeafMissingSpans` /
  `SpanOutOfBounds`. The structural errors should never fire post-
  ADJ01/ADJ02 in a healthy pipeline; they're defence-in-depth.

For each leaf node (Fact / Query / Uncertainty / Rule / Exception):

1. Build a textual node description (id + kind + polarity + modality
   + term-Debug).
2. Call `render_node` → faithful paraphrase.
3. Call `entail(premise=source, hypothesis=rendering)` →
   `p_to_h_score`.
4. From the same `entail` call: `h_to_p_score` (entail is
   bidirectional).
5. If either score `< threshold` → record a `RoundTripDrift` violation.

Skips TextRun (grouping) and Discarded nodes. ADJ04's job is leaf-
level fidelity; cross-node coherence is ADJ02's job.

11 tests cover: zero-node-types-to-check happy path; high-scoring
pass; source→rendering drift (rendering claims more); rendering→source
drift (IR misses content); custom threshold respected; missing
Renderer client → `CheckError::Primitive`; out-of-bounds span and
leaf-missing-spans typed errors; multi-span concatenation with `…`
separator; one render+entail call record per node, interleaved;
Discarded nodes skipped.

### Notes

The checker writes its LlmCallRecord stream into `RoundTripResult.call_records`
so the pipeline can copy them into `CheckerResult` for the audit
trail. v0.2 will plumb the records into `CheckerResult.telemetry`
directly via an `adjudication-pipeline` integration commit.

Reference: [ADJ04](../../../specs/ADJ04-round-trip-checker.md).
