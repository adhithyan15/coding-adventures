# ADJ22 — Typed-quantity coverage checker

## Overview

ADJ22 is the deterministic-Rust companion to
[ADJ21](ADJ21-typed-quantity-decomposition.md). Where ADJ21
updates `decompose_text`'s prompt to teach the LLM to emit
`quantity(value, unit)` compounds for every numerical literal,
ADJ22 is the **validator that catches when the model drops a
quantity anyway** — either by omitting it, flattening it into
the predicate name, or losing the unit.

The check is structurally identical to ADJ02 coverage: walk a
property of the source, walk the IR, verify mappings. ADJ02
verifies every byte is covered by some span. ADJ22 verifies
every numerical literal is covered by some `quantity(...)`
compound.

## Why it exists

ADJ18's empirical bench (v0.13 prompts, PR #3066) showed that
all five models in the lineup mishandled the "4-inch blade vs
2.36-inch threshold" comparison. The structural fix is to take
the LLM out of the arithmetic loop entirely — let the engine
evaluate threshold comparisons by giving it typed quantities in
the source IR.

[ADJ21](ADJ21-typed-quantity-decomposition.md) lands the
prompt-side of that fix: the `decompose_text` system prompt
explicitly teaches the model to produce `quantity(value, unit)`
compounds. But prompts aren't contracts — small models routinely
ignore instructions, drop fields, or flatten structures.

ADJ22 turns the ADJ21 contract into a hard check the framework
enforces pre-engine. If the LLM drops a quantity, the
clarification dialogue (ADJ06) re-prompts. If the LLM keeps
dropping it, the framework's audit trail records the failure
rather than producing a verdict over incomplete IR.

## What the checker does

```rust
pub fn check_typed_quantity_coverage(
    doc: &Document,
    ir_doc: &IRDocument,
) -> TypedQuantityResult;
```

Algorithm:

1. **Scan `doc.normalized_text` for numerical literals.** A
   literal is `\d+(\.\d+)?` — one or more ASCII digits, optionally
   followed by `.` and more digits. `"4"`, `"3.4"`, `"750"`,
   `"200"` are literals; `"-5"`, `"5e10"` are not (negative
   numbers and scientific notation are out of scope; the few
   adjudication cases that need them can be added later).

2. **For each literal**, find IR nodes whose `source_spans`
   overlap the literal's byte range. Only `NodeKind::Fact`,
   `NodeKind::Rule`, and `NodeKind::Uncertainty` are considered;
   Section / Entity / Query / Discarded / Exception nodes are
   exempt (their terms aren't expected to carry source-level
   quantities).

3. **Check that at least one overlapping node has a
   `quantity(<lit>, _)` compound** somewhere in its term tree.
   The walk is recursive — `quantity(4, inches)` nested inside
   `blade_length(knife, ...)` inside
   `meets_threshold(blade_length(...))` is found.

4. **Value matching is normalisation-aware**:
   - `"4"` matches `Term::Atom("4")`, `Term::Num(Int(4))`,
     `Term::Num(Float(4.0))`.
   - `"4.0"` and `"4"` both canonicalise to `"4"`.
   - `"04"` (with leading zero) canonicalises to `"4"`.
   - `"0.5"` keeps its leading zero (decimal-prefix preservation).
   - `"3.4"` matches `Term::Atom("3.4")` and `Term::Num(Float(3.4))`.

5. **Unit is not validated in this iteration.** ADJ22 v0.1 only
   verifies that *some* `quantity(<lit>, _)` exists. A future
   iteration could enforce per-domain unit vocabularies if needed
   (e.g., "for clinical, expected units include `celsius`,
   `mmHg`, `bpm`").

## Violation shape

```rust
pub enum TypedQuantityViolation {
    MissingQuantity {
        literal: String,                          // "4"
        location: (usize, usize),                 // byte range in source
        nearby_nodes: Vec<adjudication_ir::NodeId>,  // overlapping nodes
    },
}
```

`nearby_nodes` is the load-bearing field for ADJ06: the
clarification prompt can quote *"You produced N1 over the range
that contains the literal '4', but its term did not include a
`quantity(4, _)` compound. The downstream rule needs the typed
value to compare against its threshold. Please re-extract."*

## Why this is in `adjudication-coverage`, not its own crate

The existing convention in the framework has one crate per
checker (ADJ02 → `adjudication-coverage`, ADJ03 →
`adjudication-polarity-modality`, etc.). ADJ22 is structurally a
coverage check — it just checks coverage of a different property
(numerical literals) against a different IR structure (quantity
compounds). Putting it in the same crate as ADJ02 keeps the
"coverage" namespace coherent and avoids a one-file crate for
what is a ~250-line addition.

If ADJ22 grows substantially (per-domain unit vocabularies,
complex literal scanners, etc.) it can be extracted to its own
crate later. For v0.1 it lives next to the byte-coverage check.

## What ADJ22 does NOT do

- **Doesn't enforce unit atoms.** `quantity(4, "snorgles")` passes
  the check even though `snorgles` is not a real unit. The point
  is to catch the model dropping quantities entirely, not to
  enforce a domain-specific unit vocabulary.
- **Doesn't fix the IR.** The check is read-only. ADJ06's
  clarification dialogue is where re-extraction happens.
- **Doesn't run automatically yet.** ADJ22 is a standalone
  function callable today; wiring it into
  `adjudication-pipeline` so failures route to ADJ06 is a
  follow-up (probably a separate small PR).

## Implementation

`adjudication-coverage` v0.3.0 adds:

- `TypedQuantityViolation` enum (one variant for now).
- `TypedQuantityResult { Pass, Fail { violations } }`.
- `check_typed_quantity_coverage(doc, ir_doc) -> TypedQuantityResult`
  — the public entry point.
- Internal helpers: `scan_numerical_literals`, `spans_overlap`,
  `term_contains_quantity`, `atom_or_num_matches_literal`,
  `normalise_numeric`.

13 unit tests cover:

- Literal scanning: integers, decimals, multiple values, none.
- Pass cases: top-level quantity, decimal value, numeric-atom
  value, deeply-nested quantity, no-numbers-in-source.
- Fail cases: missing quantity, flattened-into-predicate
  (`blade_4_inches`), multiple missing literals.
- Normalisation: leading zeros, trailing decimal zeros,
  `0.5`-edge case.

## What this unlocks

Once ADJ22 is wired into the pipeline (a follow-up), the
ADJ21 contract becomes enforceable end-to-end:

```
source text
   ↓ decompose_text v5 (ADJ21 prompt)
IR with quantity(value, unit) compounds
   ↓ check_coverage (ADJ02)
every byte covered
   ↓ check_typed_quantity_coverage (ADJ22, this PR)
every numerical literal preserved as a typed quantity
   ↓ check_propagation (ADJ03)
polarity/modality consistent
   ↓ ... other checks ...
   ↓ engine
deterministic verdict using typed quantity comparisons
```

The pocket-knife regression that motivated ADJ21 and ADJ22 is
gone end-to-end: the LLM is no longer asked to evaluate
`4 > 2.36` inside its forward pass. The IR carries
`quantity(4, inches)`, the rulebook says
`gt(L, quantity(2.36, inches))`, the engine unifies and
evaluates.

## Pipeline integration (follow-up)

The wiring change to `adjudication-pipeline`'s
`run_with_gateway`:

```rust
// After existing ADJ02 coverage check:
let q_started = now();
let q_result = check_typed_quantity_coverage(&cov_doc, &input.ir_document);
let q_completed = now();
trail.checker_results.push(/* ADJ22 result */);

let prior_gating_ok =
    matches!(cov_result, CoverageResult::Pass)
    && matches!(q_result, TypedQuantityResult::Pass)
    && pm_result.pass();
```

ADJ22 failures route to ADJ06 the same way ADJ02 failures do:
the clarification prompt names the missing literal and the node
that should have included it. The retry loop already exists; the
new prompt template is the only new piece.

That wiring lives in a separate PR alongside the
`retry_decompose_on_missing_quantity` clarification primitive.

## See also

- [ADJ21](ADJ21-typed-quantity-decomposition.md) — the prompt
  side of the same change.
- [ADJ02](ADJ02-coverage-checker.md) — the byte-coverage check
  this one parallels.
- [ADJ06](ADJ06-clarification-dialogue.md) — the clarification
  loop that consumes ADJ22 violations.
- [ADJ18 §"v0.13 re-run results"](ADJ18-broadened-tsa-empirical-bench.md)
  — the empirical evidence motivating both ADJ21 and ADJ22.
- ADJ23 (queued) — empirical re-bench against v5 decompose_text
  + ADJ22 enforcement.
