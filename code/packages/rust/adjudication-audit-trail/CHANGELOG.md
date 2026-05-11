# Changelog

All notable changes to this project will be documented in this file.

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
