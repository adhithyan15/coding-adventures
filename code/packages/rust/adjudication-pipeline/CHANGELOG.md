# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-05-11

### Added

End-to-end orchestrator for the adjudication framework. Composes the
merged checker passes + the engine connector + the audit-trail
schema into a single function.

- `run(input, adjudication_id, now)` — one-call orchestrator. Runs
  ADJ02 (coverage) + ADJ03 (polarity-modality), records both into
  the audit trail, then runs the engine connector if (and only if)
  every gating check passed.
- `PipelineInput { document: PipelineDocument, ir_document: IRDocument }`
  — minimal input. `PipelineDocument` carries id / name / received_at /
  normalized_text / normalization metadata so callers don't have to
  import `adjudication-coverage` just to build an input.
- `PipelineOutput { verdict, audit_trail }`.
- `Verdict::Resolved { answers }` / `Verdict::Blocked { violation_count }` /
  `Verdict::EngineError(String)`.
- ADJ04 (round-trip) and ADJ05 (adversarial) are recorded as
  `PassOutcome::Skipped` with `pass_version = "not-yet-wired"` so
  the trail shape is complete and the slots are ready for those
  checkers to fill in.

7 unit tests cover: empty IR + empty text resolves cleanly with all
four checker results recorded; an out-of-bounds span surfaces as a
coverage violation, blocks the engine, and populates the audit
trail's `ClarificationExhausted` outcome; input document
normalization metadata is mirrored into `AuditTrail.documents`; schema
version stamp is recorded; IR nodes are mirrored into
`AuditTrail.ir_nodes`; every checker result carries a non-empty
`pass_version`; the full `AuditTrail` round-trips through serde_json.

### Notes

This is the **semantic source map running end-to-end** for the slices
of the framework that exist today. ADJ04 (round-trip) and ADJ05
(adversarial) need their own checker crates plus the
`find_contradicting_reading` primitive before they can slot in;
those land in follow-ups. The pipeline's public surface
(`PipelineInput`, `PipelineOutput`, `Verdict`) is designed to stay
stable across those additions — only the `Skipped` entries flip to
`Passed`/`Failed` as each checker comes online.

Extraction (LLM source-text → IR) lives a layer below: v0.2 will
wire `llm_primitives::decompose_text` in front so the input shrinks
to `(source_text: String, doc_id: DocumentId)`.

Reference: [ADJ00](../../../specs/ADJ00-adjudication-framework.md),
[ADJ07](../../../specs/ADJ07-audit-trail-schema.md), and the
[ADJ10 TSA worked example](../../../specs/ADJ10-tsa-worked-example.md).
