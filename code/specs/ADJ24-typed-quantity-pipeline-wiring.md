# ADJ24 — Typed-quantity pipeline wiring

## Overview

ADJ24 closes the loop opened by
[ADJ21](ADJ21-typed-quantity-decomposition.md) (decompose_text v5
prompt) and [ADJ22](ADJ22-typed-quantity-coverage.md) (validator).
ADJ22 already detects when the LLM dropped a numerical literal;
this spec wires that detection into the pipeline so a failure
triggers an ADJ06 clarification re-prompt the same way an ADJ02
coverage failure already does.

[ADJ23](ADJ23-decomposition-bench.md) measured 10% first-pass
ADJ22 across the 8×5 matrix, with the dominant failure being the
"1 carry-on bag" count quantity dropped in 92.5% of cells.
ADJ23's "workstream A" identified the retry loop as the highest-
priority follow-up — llama3.1's 40% recall would plausibly climb
toward 80% with one clarification re-prompt for the dropped count.

## What ADJ24 ships

Four crate-level changes that together make the typed-quantity
contract enforceable end-to-end:

### 1. `adjudication-audit-trail` v0.x → v0.x+1

- Add `PassName::Adj22TypedQuantity` enum variant.
- Add `ClarificationKind::MissingQuantity` variant.

Both are purely additive — existing audit-trail consumers that
match on `_ => …` still compile.

### 2. `adjudication-clarification` v0.x → v0.x+1

New public surface, mirroring the existing coverage and polarity
retry primitives:

```rust
pub struct TypedQuantityClarificationRequest {
    pub original: DecomposeTextRequest,
    pub violation_description: String,
    pub previous_ir: serde_json::Value,
    /// One per missing literal (small N — most decls have ≤3).
    pub missing_literals: Vec<MissingLiteralHint>,
}

pub struct MissingLiteralHint {
    pub literal: String,                 // "4"
    pub source_byte_range: (usize, usize),
    pub nearby_node_ids: Vec<String>,
}

pub const TYPED_QUANTITY_CLARIFICATION_PROMPT_VERSION:
    &str = "typed-quantity-clarification-v1";

pub fn retry_decompose_on_typed_quantity_failure(
    req: &TypedQuantityClarificationRequest,
    gateway: &GatewayConfig,
    max_attempts: usize,
    now: impl Fn() -> String,
) -> Result<CoverageClarificationOutcome, ClarificationError>;
```

The correction prompt names each missing literal, points at the
source byte range that contains it, and reminds the model that
counts (`1 carry-on bag`) must be wrapped in
`quantity(1, count)` rather than left as bare atoms — the
single most common failure mode from ADJ23.

The function reuses `retry_with_correction_prompt` (the shared
inner loop already used by the coverage and polarity retries) so
the retry semantics, dialogue-turn shape, and max-attempts
behaviour stay aligned.

### 3. `adjudication-pipeline` v0.x → v0.x+1

- Import `check_typed_quantity_coverage` +
  `TypedQuantityResult` + `TypedQuantityViolation` from
  `adjudication-coverage` (already at v0.3.0).
- Insert the call into `run_with_gateway` after ADJ03
  polarity/modality, before the ADJ04 gate:

  ```rust
  // ---------- ADJ22 typed-quantity coverage ----------
  let tq_started = now();
  let tq_result = check_typed_quantity_coverage(&cov_doc, &input.ir_document);
  let tq_completed = now();
  trail.checker_results.push(typed_quantity_to_checker_result(
      tq_started, tq_completed, &tq_result,
  ));
  ```

- Add `typed_quantity_to_checker_result` helper (sibling to
  `coverage_to_checker_result`), mapping each
  `TypedQuantityViolation::MissingQuantity` to a `Violation` with
  `pass_name = Adj22TypedQuantity`, `kind = MissingQuantity`, and
  a `detail` JSON object carrying `{ literal, location:
  [start,end], nearby_nodes: [...] }`.
- Add ADJ22 to the engine-gating condition: the engine only runs
  when `cov_result == Pass && pm_result.pass() && tq_result == Pass`.
- Use the same gating to skip ADJ04/ADJ05 when ADJ22 fails (no
  point paying for the round-trip / adversarial LLM calls if the
  IR doesn't carry typed quantities yet).

### 4. `adjudication-tsa-demo` v0.x → v0.x+1

Extend the existing clarification loop (currently handles ADJ02
and ADJ03) with a third branch:

- Priority order: **ADJ02 (coverage) → ADJ22 (typed-quantity) →
  ADJ03 (polarity)**. Coverage first (structural — without it
  spans are unreliable); typed-quantity second (typing —
  enables engine arithmetic); polarity third (semantic
  refinement on a structurally valid IR).
- On an ADJ22-only failure, build a
  `TypedQuantityClarificationRequest` from the violations and
  call `retry_decompose_on_typed_quantity_failure`.
- Re-run the entire pipeline against the corrected IR (same
  pattern as today's ADJ02 / ADJ03 retries) so every checker
  sees the new IR.

## Why this priority order

ADJ22 sits between ADJ02 and ADJ03 because:

- **ADJ02 first** — if the IR doesn't tile the source, ADJ22's
  `nearby_nodes` computation (which uses overlapping source spans)
  reports misleading information. Fix coverage before you try to
  reason about which node "owns" a missing literal.
- **ADJ22 before ADJ03** — typed quantities are a structural
  property of the IR shape, the same way coverage is. ADJ03's
  polarity / modality concerns are layered on top of a
  structurally-correct IR. Asking the model to fix polarity on
  an IR that drops half its numerical literals wastes a turn.

## Why the clarification prompt names each missing literal

ADJ23's results showed that small models can extract some typed
quantities while dropping others (llama3.1:8b: 100% measurement
recall, 0% count recall). A correction prompt that says only
*"add typed quantities"* is no more useful than the v5 system
prompt — the model already saw that instruction. A prompt that
says *"You produced N1 over the range '1 carry-on bag' but its
term does not include `quantity(1, _)`. Add it."* is targeted
feedback the model can act on.

## Correction prompt template

```
Your previous IR was REJECTED by the ADJ22 typed-quantity checker.

Missing typed quantities:
  - literal "1" at bytes 0..1 (covered by node N1)
    → wrap as `quantity(1, count)` for counts of items
  - literal "4" at bytes 15..16 (covered by node N2)
    → wrap as `quantity(4, <unit>)` where <unit> reflects the
      surrounding context (in/inch/inches, oz, ml, etc.)

The typed-quantity rule is non-negotiable: every numerical literal
in SOURCE must appear inside a `quantity(value, unit)` compound
term somewhere in the IR. Flattening the literal into the predicate
name (e.g., `blade_4_inches`) is REJECTED — the engine needs the
typed value to compare against rule thresholds.

Common units:
  - count       (for bag counts, item counts: `quantity(1, count)`)
  - inch / inches, mm, cm, ft  (length)
  - oz, ml, l, gallons         (volume)
  - g, kg, lb                  (mass)
  - wh, kwh, mAh, v            (electrical)
  - celsius, fahrenheit, k     (temperature)
  - bpm, mmHg                  (clinical)

Your previous output was:
{previous_pretty_ir}

Produce a CORRECTED IR with the same `document_id`, fixing the
missing quantities. Keep the same shape; you may add new nodes
to host the new quantity compounds, or wrap existing atoms.
```

The prompt is domain-neutral by design — examples cover counts,
length, volume, mass, electrical, temperature, clinical. No
TSA / clinical / contract specifics inside the framework
instructions.

## Tests

Each crate adds tests local to its change:

- `adjudication-audit-trail`: round-trip a `CheckerResult` with
  `pass_name = Adj22TypedQuantity` and a `Violation` with
  `kind = MissingQuantity`.
- `adjudication-clarification`:
  - Smoke: `retry_decompose_on_typed_quantity_failure` runs once
    and returns a corrected IR (against a stub gateway).
  - Prompt content: the correction prompt names each missing
    literal and contains the "wrap as quantity" instruction.
  - Domain-neutrality: regression-guard test that the framework
    prompt does NOT contain TSA / clinical / contract specifics.
  - Exhaustion: `max_attempts = 1` and gateway always fails →
    `ClarificationError::Exhausted` with one dialogue turn.
- `adjudication-pipeline`:
  - When the IR has a `quantity(4, inches)` compound covering the
    only literal, the new ADJ22 checker_result has
    `PassOutcome::Passed`.
  - When the IR drops a literal, the checker_result has
    `PassOutcome::Failed` and one `Violation` per missing literal
    with `pass_name = Adj22TypedQuantity` and
    `kind = MissingQuantity`.
  - Gating: if ADJ22 fails but ADJ02 and ADJ03 pass, ADJ04 is
    skipped.
- `adjudication-tsa-demo`:
  - Existing tests stay green.
  - New test: when the LlmExtracted IR has an ADJ22 failure (no
    quantity for a literal) and ADJ02 / ADJ03 pass, the demo
    calls the typed-quantity retry primitive.

## Out of scope (deferred)

- **Per-domain unit-vocabulary enforcement** — ADJ22 v0.1 doesn't
  validate the unit atom (`quantity(4, snorgles)` passes). A
  future ADJ22.x can enforce that the unit comes from a
  per-domain vocabulary. Not in ADJ24.
- **Fact-sheet-based count handling** — ADJ23 noted that
  declaration counts ("1 carry-on bag") may be better handled as
  a deterministic field on a fact-sheet record than asking the
  LLM to extract them. That's the ADJ20 workstream, separate PR.
- **gemma4 JSON-emission cap** — ADJ23 noted that gemma4 trunc-
  ates mid-string on ~50% of cells. That's a gateway-level
  investigation, separate from ADJ24's pipeline wiring.

## Validation

After ADJ24 lands, re-run the ADJ23 bench with the demo's
clarification loop enabled (`ADJ_DEMO_MAX_CLARIFY_ATTEMPTS=3` —
the existing knob). The expected delta:

- llama3.1:8b: recall 40% → ~80% (the count branch gets a
  second pass with explicit feedback).
- qwen2.5 sizes: smaller absolute improvement but recall should
  climb non-trivially as the prompt targets one specific missing
  literal rather than the whole v5 contract.
- gemma4:latest: unaffected by ADJ22 retries — gemma4's failures
  are gateway-level JSON truncation, not missing quantities.
  Workstream B (separate PR) addresses gemma4.

Results recorded as ADJ23 v2 bench data in a follow-up commit.

## See also

- [ADJ21](ADJ21-typed-quantity-decomposition.md) — decompose_text
  v5 prompt.
- [ADJ22](ADJ22-typed-quantity-coverage.md) — the validator.
- [ADJ23](ADJ23-decomposition-bench.md) — empirical findings
  motivating ADJ24.
- [ADJ06](ADJ06-clarification-dialogue.md) — the retry-loop
  contract this wiring plugs into.
