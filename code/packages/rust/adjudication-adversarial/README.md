# adjudication-adversarial (Rust)

ADJ05 adversarial verifier. For each sampled leaf IR node: render →
find contradicting reading → judge plausibility. Plausible
contradictions become `AdversarialReading` violations.

## Usage

```rust
use adjudication_adversarial::{check_adversarial, CheckOptions};
use llm_primitives::{GatewayConfig, RenderStyle};

// Gateway must have Role::Renderer + Role::Adversary + Role::Plausibility.
// IMPORTANT: Role::Adversary must be a different model family from
// Role::Extractor — enforced at startup by GatewayConfig::check_independence.

let result = check_adversarial(
    document_text,
    &ir_document,
    &gateway,
    &CheckOptions {
        style: RenderStyle::Plain,
        domain_hint: "tsa-declaration".into(),
    },
)?;

for v in &result.violations {
    // v.node_id, v.ir_rendered, v.adversary_reading,
    // v.adversary_explanation, v.judge_reason
}

// result.call_records carries one record per LLM call (2 or 3 per
// node depending on whether the adversary concurred or not).
```

## The three-step loop

1. **`render_node`** translates the IR node back into text.
2. **`find_contradicting_reading`** asks the Adversary for the
   strongest alternative reading of the source that contradicts the
   IR. Returns either `Concurs` or `Reading { text, explanation }`.
3. **`judge_plausibility`** is consulted only when the adversary
   found a reading: would a competent practitioner adopt it?

Outcomes:

- **Concurs** → no violation, only 2 LLM calls recorded.
- **Reading + IMPLAUSIBLE** → no violation; the reading goes into
  `call_records` for the audit trail. (Per ADJ05: implausible
  contradictions are still logged.)
- **Reading + PLAUSIBLE** → an `AdversarialViolation` is recorded.
  ADJ06 picks it up to clarify with the user.

## Independence requirement

ADJ05's whole point is that the Adversary must be a *different
model family* from the Extractor. Enforced at startup via
`GatewayConfig::check_independence`. The checker does not
double-check at every call — a redundant check would just slow
every invocation.

## What v0.1 does not do

- **Sample.** Visits every leaf node. ADJ05's
  `adversary_sample_rate` knob is a follow-up.
- **Retry on primitive validation failure.** Surfaces as
  `CheckError::Primitive`. A future retry harness can wrap.
