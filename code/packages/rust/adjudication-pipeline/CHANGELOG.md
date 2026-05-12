# Changelog

All notable changes to this project will be documented in this file.

## [0.4.0] - 2026-05-12

### Added

ADJ05 adversarial checker wired in. The pipeline now runs
`adjudication-adversarial::check_adversarial` when:

1. A `GatewayConfig` is supplied, AND
2. `Role::Adversary` is registered, AND
3. The Extractor and Adversary clients come from different
   `(vendor, model_family)` pairs (LM00b independence requirement,
   enforced by `GatewayConfig::check_independence`).

If any of these conditions fails, ADJ05 records as `Skipped` with a
human-readable `skipped_reason` in `telemetry`. If the checker itself
errors (gateway transport failure, primitive validation exhausted,
…), the pipeline records `Failed` with the error string in
`telemetry.check_error` — same pattern ADJ04 uses.

- `Adj05Decision` enum with `Skipped { reason }` / `Ran(result)` /
  `CheckErrored(detail)` variants.
- `run_adj05` / `adj05_to_checker_result` / `adversarial_violation_to_audit`
  helpers next to the ADJ04 equivalents.
- ADJ05 violations carry `ClarificationKind::AdversarialReading`
  with `ir_rendered`, `adversary_reading`, `adversary_explanation`,
  and `judge_reason` in the detail JSON.
- ADJ05 is **advisory** at v0.4 — drift records as Failed but the
  engine still runs. A future ADJ06 wiring can gate on it.
- Two new tests cover ADJ05: skip-when-no-gateway and
  skip-with-reason-when-Adversary-role-missing. The Ran/CheckErrored
  paths are exercised end-to-end by the demo's live integration
  against Ollama.

## [0.3.0] - 2026-05-11

### Added

ADJ04 round-trip checker wired in via a new `GatewayConfig` argument.
When a caller supplies `Renderer` + `Nli` clients, the pipeline now
runs `adjudication-round-trip::check_round_trip` and records the
result in the audit trail with `pass_version = "v1.0"`. When the
gateway is omitted (or roles are missing), the v0.2 behaviour is
preserved — ADJ04 records as `Skipped`.

- New entry point `run_with_gateway(input, id, now, gateway)` —
  the v0.3 preferred surface.
- Existing `run(input, id, now)` is unchanged on the wire and now
  delegates to `run_with_gateway(_, _, _, None)`, so v0.2 callers
  recompile without source changes.
- ADJ04 is **advisory** at v0.3 — a failing round-trip records as
  `PassOutcome::Failed` with structured violations
  (`ClarificationKind::RoundTripDrift`) but does NOT block the
  engine. ADJ06 clarification (a future PR) will gate on drift.
- Round-trip is **not run** when ADJ02 or ADJ03 already failed —
  no point burning tokens to re-discover what the deterministic
  checkers already proved.
- Round-trip checker errors (missing role, validation exhaustion,
  transport failure) surface as `Failed` with the error string in
  `telemetry["check_error"]` rather than panicking.
- 5 new unit tests cover: high-score pass, drift-fails-but-engine-
  still-runs, no-gateway-records-Skipped, missing-role-records-
  Failed-with-detail, prior-fail-skips-ADJ04.

### Notes

ADJ05 still records as `Skipped`. It needs a second `Adversary`
client from a different `(vendor, model_family)` than the `Extractor`
to satisfy the LM00b independence requirement; that arrives once a
second model family is wired into the deployment.

This is also the first piece that lets the framework be driven by a
**local Ollama instance** end-to-end — a deployment with two locally
served models (e.g. `gemma:7b` for `Renderer`, a separate family
like `llama3.1:8b` for `Nli`) can now exercise ADJ04 without any
cloud LLM access.

## [0.2.0] - 2026-05-11

### Added

ADJ10 TSA worked-example integration test (the third E2E goal of
the session: Prolog ✅, ProbLog ✅, semantic source map ✅).

- New integration test crate `tests/integration_adj10_tsa.rs`. Builds
  a TSA-style IR document programmatically (two `Fact` nodes that
  tile a `"1 carry-on bag, matches."` 24-byte document plus one
  `Query` node) and feeds it through `pipeline::run`.
- 4 tests cover: the happy path (Resolved verdict with one engine
  answer, all four checker results recorded — ADJ02/ADJ03 Passed,
  ADJ04/ADJ05 Skipped); audit trail round-trips through
  `serde_json`; the trail mirrors the input document and every IR
  node id; the Blocked path (out-of-bounds span surfaces as a
  coverage violation, engine never runs, outcome is
  `ClarificationExhausted`).

### Notes

This is the **third of three E2E test goals** the user asked for at
the start of the session, alongside [#2752 Prolog](https://github.com/adhithyan15/coding-adventures/pull/2752)
and [#2756 ProbLog](https://github.com/adhithyan15/coding-adventures/pull/2756).
The fixture is programmatic at v0.2 because `adjudication-ir` does
not yet derive `serde::Deserialize`; a future version will load the
ADJ10 fixture from a JSON file under
`code/specs/fixtures/adj10-tsa/`.

A follow-up will pass an LLM `GatewayConfig` into `run` so ADJ04
(round-trip) and ADJ05 (adversarial) can flip from Skipped to
Passed/Failed using the merged `adjudication-round-trip` and
`adjudication-adversarial` checker crates.

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
