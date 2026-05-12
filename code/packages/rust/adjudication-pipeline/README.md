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
   pipeline::run_with_gateway(input, adjudication_id, now, gateway?)
        │
        ├── adjudication-coverage::check_coverage()          (ADJ02 v2)
        ├── adjudication-polarity-modality::check_propagation()  (ADJ03 v2)
        ├── adjudication-round-trip::check_round_trip()      (ADJ04 v1, advisory; needs Renderer+Nli)
        ├── [ADJ05 adversarial — Skipped until a family-disjoint Adversary client is wired]
        ├── adjudication-connector::run_adjudication()       (engine)
        └── writes everything into adjudication-audit-trail::AuditTrail
```

## What v0.3.0 ships

- `run(input, adjudication_id, now) → PipelineOutput` — unchanged
  wire-compatible entry for v0.2 callers; delegates to
  `run_with_gateway(_, _, _, None)`.
- `run_with_gateway(input, adjudication_id, now, gateway: Option<&GatewayConfig>)` —
  v0.3's preferred entry point. Pass `Some(&g)` to enable ADJ04.
- `PipelineInput { document, ir_document }`.
- `PipelineDocument` — minimal struct carrying id, name, received_at,
  normalized_text, and normalization-pipeline metadata.
- `PipelineOutput { verdict, audit_trail }`.
- `Verdict::Resolved { answers }` / `Blocked { violation_count }` /
  `EngineError(String)`.
- ADJ04 is **advisory** at v0.3: drift records as `Failed` with
  structured `RoundTripDrift` violations but the engine still runs.
  A future ADJ06 wiring will gate on it.
- ADJ05 remains `Skipped` pending a family-disjoint adversary.

## What v0.3.0 does NOT ship

- **Extraction** — today's pipeline accepts a pre-built `IRDocument`.
  A follow-up will add a `run_from_source(source_text, gateway, …)`
  entry point that calls `llm_primitives::decompose_text` in front.
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
