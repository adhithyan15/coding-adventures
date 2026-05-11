# adjudication-audit-trail (Rust)

JSON-serializable audit-trail schema for the adjudication framework.
Reference implementation of
[ADJ07 — Audit Trail Schema](../../../specs/ADJ07-audit-trail-schema.md).

## What This Is

> The IR is the audit trail. Every fact in the engine, every rule
> that fires, every clarification turn, every checker-pass decision,
> every proof step — all of these chain back without gaps to the
> source bytes of the original document. The chain is **data**, not
> commentary.

This crate is the *shape* of that chain. Pure data types with
`serde::{Serialize, Deserialize}` derives — no I/O, no behaviour
beyond round-tripping through JSON. Producers (the checker passes,
the engine, the dialogue layer) build up an [`AuditTrail`] as the
adjudication runs; the trail is then handed to whatever persistence
layer the deployment uses (inline response, append-only log,
content-addressed storage).

## Where It Fits

```text
   adjudication-coverage  -.
   adjudication-polarity   |
   adjudication-round-trip |---> audit-trail (this crate) ---> persistence
   adjudication-adversary  |
   logic-engine           -'
   dialogue                /
   primitives             /
```

## Public API

| Item | Role |
|---|---|
| `AuditTrail` | Top-level. Carries id, timestamps, outcome, documents, IR nodes, checker results, dialogue, engine artifacts, configuration, schema version |
| `AdjudicationId` / `DocumentId` / `NodeId` / `TurnId` | Stable string / integer identifiers |
| `Document`, `NormalizationRecord`, `AppendInfo` | Input documents with byte-offset provenance |
| `IrNode` | Per-node payload (opaque `serde_json::Value` at v0.1; typed at v0.2) |
| `CheckerResult`, `Violation`, `PassName`, `PassOutcome`, `ClarificationKind` | Checker-pass results and per-violation detail |
| `DialogueTurn`, `DialogueRung`, `DialogueResponse`, `DialogueOutcome`, `DialogueResponseSource` | ADJ06 dialogue records |
| `EngineArtifacts`, `KbSummary`, `BooleanFormula`, `WmcResult`, `SearchMode` | LP19 engine output + probabilistic-inference artifacts |
| `Configuration`, `VersionedComponent` | Reproducibility-relevant configuration |
| `AdjudicationOutcome` | `InProgress` / `Resolved` / `ClarificationExhausted` / `Aborted` / `TimedOut` |
| `AppendedRecord` | Optional content-addressed chaining for tamper-evidence |

## Why IR nodes are opaque at v0.1

`adjudication-ir`'s IR types don't yet derive `Serialize` /
`Deserialize`. Rather than block the audit-trail schema on that
refactor, v0.1 stores each `IrNode.payload` as a `serde_json::Value`.
v0.2 will replace the payload type with `adjudication_ir::IRNode`
once that crate gains serde derives. The **on-wire JSON shape stays
the same** — only the static type signature changes — so consumers
who serialize their typed IR via `serde_json::to_value` today won't
break tomorrow.

## Usage

```rust
use adjudication_audit_trail::*;

let mut trail = AuditTrail::new(
    AdjudicationId::new("adj-1"),
    "2026-05-11T08:00:00Z",
);

trail.documents.push(Document {
    id: DocumentId::new("doc1"),
    name: "tsa_declaration".into(),
    received_at: "2026-05-11T08:00:00Z".into(),
    normalized_text: "1 carry-on bag, 1 personal item.".into(),
    normalization: NormalizationRecord {
        pipeline: "plain-text-v1".into(),
        version: "1.0.0".into(),
        options: Default::default(),
    },
    raw_base64: None,
    appended_turns: Vec::new(),
});

trail.outcome = AdjudicationOutcome::Resolved {
    answer: serde_json::json!({"allowed": true}),
};
trail.completed_at = Some("2026-05-11T08:00:05Z".into());

let json = serde_json::to_string_pretty(&trail).unwrap();
```

## JSON Conventions

- Enums serialize with `#[serde(rename_all = "snake_case")]` (e.g.
  `PassName::Adj04RoundTrip` → `"adj04_round_trip"`).
- `AdjudicationOutcome` is an internally-tagged enum with
  `tag = "kind"`: `{"kind": "resolved", "answer": ...}`.
- `SearchMode` uses `PascalCase` to match LP19's existing wire format.
- Optional fields (`raw_base64`, `formula`, `wmc_result`, ...) are
  omitted from JSON when `None` to keep typical trails compact.
- Vectors and maps with default-empty values omit themselves when
  empty (`#[serde(default, skip_serializing_if = "Vec::is_empty")]`).
- All deserialization is forward-compatible: missing optional or
  default-valued fields fall back to their `Default` impl.
