# adjudication-round-trip (Rust)

ADJ04 round-trip checker. For each leaf IR node: render it back into
natural language via `render_node`, then run bidirectional `entail`
between the rendering and the original source span. Drift in either
direction is a violation.

## What It Does

```rust
use adjudication_round_trip::{check_round_trip, CheckOptions};
use llm_primitives::GatewayConfig;

let result = check_round_trip(
    document_text,
    &ir_document,
    &gateway,       // must have Role::Renderer + Role::Nli registered
    &CheckOptions::default(),  // threshold 0.6, RenderStyle::Plain
)?;

if result.pass() {
    // every node round-tripped within tolerance
} else {
    for v in &result.violations {
        // v.node_id, v.rendering, v.source_excerpt,
        // v.source_to_rendering, v.rendering_to_source, v.threshold
    }
}

// result.call_records carries one LlmCallRecord per primitive
// invocation — the pipeline writes these into the audit trail.
```

## Why Bidirectional

A one-way "does the source entail the rendering?" check misses drift
in the *other* direction (the IR claiming more than the source
supports). ADJ04 catches both:

- `p_to_h_score < threshold` ⇒ source doesn't support the IR.
- `h_to_p_score < threshold` ⇒ IR claims more than the source.

Both surface as the same violation kind, with the failing direction
visible in `source_to_rendering` / `rendering_to_source`.

## What V0.1 Does Not Do

- **Pick a model.** The gateway's `Renderer` and `Nli` slots are
  bound by the deployment. ADJ04 strongly recommends they be
  different model families.
- **Retry on validation failure.** Each primitive surfaces its own
  `ValidationExhausted`; the checker propagates it as a
  `CheckError::Primitive`. A future retry harness can wrap the loop.
- **Sample.** Every leaf node is checked. Sampling for very large
  documents is a follow-up.
