# adjudication-pipeline (Rust)

End-to-end orchestrator for the adjudication framework. Composes
the merged ADJ02 + ADJ03 checkers + the engine connector + the
ADJ07 audit-trail schema into a single function.

## What This Is

`adjudication_pipeline::run(input, adjudication_id, now)` takes a
normalized document plus a hierarchical IR document and returns a
typed `Verdict` (Resolved / Blocked / EngineError) together with a
fully-populated `AuditTrail`. The trail records the input document,
every IR node, every checker-pass result (Pass / Fail / Skipped),
the engine artifacts when the engine ran, and the configuration
that produced them.

## Where It Fits

```text
   pipeline::run(input, adjudication_id, now)
        │
        ├── adjudication-coverage::check_coverage()          (ADJ02 v2)
        ├── adjudication-polarity-modality::check_propagation()  (ADJ03 v2)
        ├── [ADJ04 round-trip — recorded as Skipped until checker ships]
        ├── [ADJ05 adversarial — recorded as Skipped until checker ships]
        ├── adjudication-connector::run_adjudication()       (engine)
        └── writes everything into adjudication-audit-trail::AuditTrail
```

## What v0.1.0 ships

- `run(input, adjudication_id, now) → PipelineOutput` — the entry point.
- `PipelineInput { document, ir_document }`.
- `PipelineDocument` — minimal struct carrying id, name, received_at,
  normalized_text, and normalization-pipeline metadata.
- `PipelineOutput { verdict, audit_trail }`.
- `Verdict::Resolved { answers }` / `Blocked { violation_count }` /
  `EngineError(String)`.
- ADJ04 and ADJ05 are recorded as `PassOutcome::Skipped` with version
  string `"not-yet-wired"` so the trail shape is complete.

## What v0.1.0 does NOT ship

- **Extraction** — today's pipeline accepts a pre-built `IRDocument`.
  v0.2 will wire `llm_primitives::decompose_text` in front so the
  input shrinks to `(String, DocumentId)`.
- **ADJ06 clarification dialogue** — a failing check produces
  `Verdict::Blocked` with the violation count; the caller handles
  the conversation loop.
- **Persistence** — the pipeline returns an in-memory `AuditTrail`;
  the deployment chooses how to write it (inline response,
  append-only log, content-addressed storage).

## Usage

```rust
use adjudication_pipeline::{run, PipelineDocument, PipelineInput, Verdict};
use adjudication_audit_trail::AdjudicationId;

let input = PipelineInput {
    document: PipelineDocument {
        id:                     "doc1".into(),
        name:                   "tsa_declaration".into(),
        received_at:            "2026-05-11T08:00:00Z".into(),
        normalized_text:        "1 carry-on bag, 1 personal item.".into(),
        normalization_pipeline: "plain-text-v1".into(),
        normalization_version:  "1.0.0".into(),
    },
    ir_document: /* a pre-built adjudication_ir::IRDocument */,
};

let id  = AdjudicationId::new("adj-tsa-001");
let now = || "2026-05-11T08:00:01Z".to_string();
let out = run(input, id, now);

match out.verdict {
    Verdict::Resolved { answers }       => { /* engine answers */ }
    Verdict::Blocked  { violation_count } => { /* show ADJ06 a list */ }
    Verdict::EngineError(detail)        => { /* operator alert */ }
}

let trail_json = serde_json::to_string_pretty(&out.audit_trail).unwrap();
```

## Why `adjudication_id` and `now` are caller-supplied

The pipeline is otherwise pure: same input + same id + same
timestamps deterministically produce the same audit trail. Tests use
a counter-backed clock; deployments hand in `chrono::Utc::now()` and
a UUIDv7. Keeping these injected avoids dragging a `chrono` or
`uuid` dep through the crate.
