# Changelog

All notable changes to this project will be documented in this file.

## [0.3.0] - 2026-05-13 — ADJ22 enum variants

### Added

Two additive enum variants so ADJ22 typed-quantity coverage
(per [ADJ22](../../../specs/ADJ22-typed-quantity-coverage.md))
can be expressed in the audit trail like every other checker pass:

- `PassName::Adj22TypedQuantity` — serializes as
  `"adj22_typed_quantity"`. Slots alongside the existing
  `Adj02Coverage` / `Adj03PolarityModality` / `Adj04RoundTrip` /
  `Adj05Adversarial` variants.
- `ClarificationKind::MissingQuantity` — serializes as
  `"missing_quantity"`. Used in `Violation::kind` for the per-
  missing-literal records the pipeline emits when ADJ22 fails.

Both are purely additive — existing consumers that match on
`_ => …` still compile. v0.2 audit-trail JSON round-trips
unchanged.

### Why

Wiring ADJ22 into the pipeline (per
[ADJ24](../../../specs/ADJ24-typed-quantity-pipeline-wiring.md))
needs both variants. Putting them in the schema crate first means
no downstream crate is forced to invent its own enum or stuff the
pass-name into `telemetry`.

### Tests

`adj22_typed_quantity_pass_name_round_trips` round-trips a
`CheckerResult { pass_name: Adj22TypedQuantity, .. violations: [
Violation { kind: MissingQuantity, detail: { literal, location,
nearby_nodes } } ] }` through JSON and back, asserting the
snake_case serialization matches the schema.

## [0.2.0] - 2026-05-12

### Added

`DialogueResponse` gains two optional fields:

- `prompt_version: Option<String>` — the clarification-prompt
  template version that produced this turn, mirroring
  `LlmCallRecord::prompt_version`.
- `prompt_hash: Option<String>` — content-addressed hash of the
  prompt the LLM saw, same FNV-1a-rendered-hex format as
  `LlmCallRecord::prompt_hash`.

Together these make ADJ06 clarification turns replayable through the
same `(prompt_version, prompt_hash)` mechanism as primitive LLM
calls. The new fields are optional with serde-skip-if-none semantics,
so v0.1 audit trails round-trip unchanged.

## [0.1.0] - 2026-05-11

### Added

Reference implementation of [ADJ07](../../../specs/ADJ07-audit-trail-schema.md).
Pure data types with `serde::{Serialize, Deserialize}` derives — no
I/O, no behaviour beyond JSON round-trip.

- `AuditTrail` — top-level record carrying adjudication id, timestamps,
  outcome, documents, IR nodes, checker results, dialogue, engine
  artifacts, configuration, and schema version. `new()` constructor
  produces an in-progress trail with empty collections and
  `AdjudicationOutcome::InProgress`.
- Identifier newtypes: `AdjudicationId`, `DocumentId`, `NodeId`,
  `TurnId`. All `#[serde(transparent)]` so the JSON shape is just the
  inner value.
- `Document`, `NormalizationRecord`, `AppendInfo` — input documents
  with normalization provenance and per-turn append byte-ranges.
- `IrNode` — per-node payload stored as `serde_json::Value` at v0.1
  (will be typed `adjudication_ir::IRNode` at v0.2 once that crate
  ships serde derives; **on-wire shape unchanged across the upgrade**).
- `CheckerResult` + `Violation` + `PassName` + `PassOutcome` —
  per-checker-pass results. `PassName` serializes as
  `adj02_coverage` / `adj03_polarity_modality` / `adj04_round_trip` /
  `adj05_adversarial`.
- `ClarificationKind` — bridges to ADJ06 (UncoveredSpan,
  AmbiguousPolarity, AmbiguousModality, RoundTripDrift,
  AdversarialReading, InheritChainUnresolved, Other).
- Dialogue types: `DialogueTurn`, `DialogueRung` (Rung1ReprompT /
  Rung2SecondOpinion / Rung3Human), `DialogueResponse`,
  `DialogueResponseSource` (Llm / Human / Cached), `DialogueOutcome`
  (Resolved / Escalated / Abandoned).
- `EngineArtifacts` — LP19 output with `engine_version`,
  `search_mode` (FindFirst / EnumerateAll / AutoDetect), `kb_summary`,
  `proof_dag` (opaque `serde_json::Value` until logic-engine ships
  typed Serialize), optional `formula: BooleanFormula` and
  `wmc_result: WmcResult` for probabilistic adjudications, and the
  engine's structured answer.
- `Configuration` + `VersionedComponent` — reproducibility-relevant
  configuration. Every model (tagger, trigger taxonomy, extractor,
  renderer, NLI, adversary, judge, rendering) has a `VersionedComponent`
  slot with name, version, and free-form config map.
- `AdjudicationOutcome` enum — `InProgress`, `Resolved { answer }`,
  `ClarificationExhausted { unresolved }`, `Aborted { reason }`,
  `TimedOut`. Serialized as an internally-tagged enum with
  `tag = "kind"` and snake-case variant names.
- `AppendedRecord` — optional content-addressed chaining for
  tamper-evidence. The trail crate stores the shape; the deployment
  chooses the hash algorithm and computes hashes at append time.
- `AuditTrail::CURRENT_SCHEMA_VERSION = "ADJ07-v1"`.

### Tests (16 passing)

Coverage includes: round-trip serialization of `AuditTrail`, every
outcome variant, every pass name, dialogue turns, engine artifacts
in both minimal and probabilistic shapes, configuration with
versioned components, schema-version constant lock-down, and
forward-compatibility (missing optional fields deserialize via
`Default`).

### Notes

This is the **schema-only** v0.1.0 — no producer logic, no replay
engine. ADJ08 (replay tooling) is a planned follow-up that depends
on this crate. The checker passes (ADJ02–05) will each gain an
output type that fits into `CheckerResult.violations`.

Reference: [ADJ07](../../../specs/ADJ07-audit-trail-schema.md).
