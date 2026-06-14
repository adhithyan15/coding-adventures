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

> Bench run: 2026-05-13. ADJ24 wiring + `ADJ_DEMO_MAX_CLARIFY_ATTEMPTS=3`
> against the same 8 declarations × 5 models matrix as
> [ADJ23](ADJ23-decomposition-bench.md). Raw data:
> [`code/specs/data/adj23-decomposition-bench-2026-05-13-with-adj24-retries.json`](data/adj23-decomposition-bench-2026-05-13-with-adj24-retries.json).

### Headline delta

|                          | ADJ23 baseline | ADJ24 retries |    Δ     |
|--------------------------|---------------:|--------------:|---------:|
| ADJ22 first-attempt pass |       4/40 (10%)|     8/40 (20%)|   +10 pp |
| Typed-quantity recall    |      21/75 (28%)|    27/75 (36%)|    +8 pp |

ADJ22 pass *doubles*; recall climbs +8 pp. The breakdown by
model is sharply bimodal and tells a clean story.

### Per-model delta

| Model            | Baseline ADJ22 | With retries | Δ ADJ22 | Baseline recall | With retries recall | Δ recall |
|------------------|---------------:|-------------:|--------:|----------------:|--------------------:|---------:|
| gemma4:latest    |       4/8 (50%)|    **7/8 (88%)**| **+38 pp** |   9/15 (60%) |        **14/15 (93%)**|**+33 pp** |
| llama3.1:8b      |        0/8 (0%) |    1/8 (12%)|  +12 pp |        6/15 (40%) |        7/15 (47%) |   +7 pp |
| qwen2.5:3b       |        0/8 (0%) |     0/8 (0%)|    0 pp |        2/15 (13%) |        2/15 (13%) |    0 pp |
| qwen2.5:1.5b     |        0/8 (0%) |     0/8 (0%)|    0 pp |        2/15 (13%) |        2/15 (13%) |    0 pp |
| qwen2.5:0.5b     |        0/8 (0%) |     0/8 (0%)|    0 pp |        2/15 (13%) |        2/15 (13%) |    0 pp |

### Takeaways

**1. The retry primitive disproportionately helps gemma4
(+38 pp ADJ22, +33 pp recall).** ADJ23 reported gemma4 was
*bimodal* — it either nailed the v5 contract perfectly or
emitted unterminated JSON and silently fell back to a hand-built
fixture. ADJ24's retry loop recovers 3 of the 4 gemma4 cells
that originally JSON-truncated: the second-pass prompt is
shorter / more focused than the v5 system prompt, so it
clears whatever output-budget edge gemma4 was hitting. Two
ADJ24 hand-built-fallback cells (`small-perfume` and
`lighter-disposable`) still produced valid ADJ22-passing IR via
retry — even when decompose's first call dies, the retry
overrides the hand-built result.

**2. llama3.1:8b's improvement is modest (+12 pp ADJ22, +7 pp
recall) — well below the hypothesised 80%.** The pre-PR
hypothesis was that a retry naming the missing count would
flip llama3.1:8b from 0% to ~80% ADJ22. Empirically, llama3.1
still skips the bag count even with explicit feedback like
*"literal '1' at bytes 0..1 (covered by N1) → wrap as
quantity(1, count)"*. The count is treated as schema
decoration the model considers cosmetic; a measurement
(`quantity(50, wh)`) it nails, a count it ignores.

The one cell that did flip was `pocket-knife` (the original
motivating regression) — llama3.1 emitted both
`blade_length(pocket_knife, quantity(4, inches))` AND
`carry_on(quantity(1, count))` on the retry, where the
baseline only produced the former. So the wiring works; the
model's prior toward "counts are not measurements" is just
strong.

**3. qwen2.5 sizes are unaffected.** This was expected (per
ADJ12 / ADJ18 model-calibration findings) but worth recording:
the smaller qwen sizes don't follow the structured-output rule
better when given a second pass with explicit feedback.
Targeted intervention (grammar-constrained decoding, fine-
tuning, or just not asking them to do typed extraction) is
workstream C territory.

**4. The "1 carry-on bag" count is still the dominant residual
failure.** Across the 32 cells that still fail ADJ22 after
ADJ24 retries, **30/32 are missing the bag count literal "1"**.
This very strongly motivates **workstream C — ADJ20 fact-sheet
handling for declaration cardinality** — counts of items don't
belong in the LLM-extraction loop at all; they should be a
deterministic field on the declaration record. Clarification
doesn't move this needle because the model has a learned prior
against typing bag counts.

### What this validates

- The ADJ22 → ADJ06 wiring works end-to-end. The audit trail
  records `pass_name: adj22_typed_quantity` checker results.
  ADJ22 failures route through the new
  `retry_decompose_on_typed_quantity_failure` primitive
  alongside the existing ADJ02 / ADJ03 retries.
- Bigger models (gemma4) benefit disproportionately from
  pinpoint retries. Even on cells where decompose_text errors
  outright (JSON truncation), the retry produces a clean IR.
- The wiring is correctly *additive* — it never makes results
  worse. No cell flipped from passing → failing across the
  delta.

### What this does NOT solve

- **Counts of items.** llama3.1:8b's residual failures and
  qwen2.5's zero improvement both trace back to the
  "model doesn't want to wrap counts" prior. The retry primitive
  is the right shape for *novel* failures (model forgot to add
  a quantity) but doesn't override learned priors. ADJ20 is the
  right home for counts.
- **qwen2.5 family.** Structurally, the retry loop is
  noise-equivalent for these models — they don't internalise
  the v5 prompt well enough that pointing at one missing
  literal makes them generalise to "wrap *all* literals". Future
  work: grammar-constrained decoding for `quantity(...)`.

### Wallclock cost

The bench took ~12 minutes wallclock with retries enabled (vs
~5 minutes for ADJ23 baseline). The extra LLM call per
ADJ22-failing cell is the dominant cost: median wallclock for
gemma4 doubled (from ~75s to ~200s). For interactive use this
is acceptable — the retry budget is bounded by
`max_clarification_attempts` and the cache covers repeated
runs.

## See also

- [ADJ21](ADJ21-typed-quantity-decomposition.md) — decompose_text
  v5 prompt.
- [ADJ22](ADJ22-typed-quantity-coverage.md) — the validator.
- [ADJ23](ADJ23-decomposition-bench.md) — empirical findings
  motivating ADJ24.
- [ADJ06](ADJ06-clarification-dialogue.md) — the retry-loop
  contract this wiring plugs into.
