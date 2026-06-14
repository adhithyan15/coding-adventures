# ADJ21 — Typed quantity decomposition (decompose_text v5)

## Overview

The ADJ18 v0.13 bench (PR #3069) confirmed empirically what the
framework's design already implied: **the LLM cannot reliably
evaluate numerical thresholds inside its forward pass.** Every
model in the lineup mishandled the "4-inch blade vs 2.36-inch
limit" comparison from rule 6, and none escalated despite the
v0.13 prompt giving them that option.

The structural fix is to **let the engine do the comparisons**.
For the engine to do that, the source IR has to preserve typed
quantities — value + unit — as structured terms rather than
folding numbers into predicate names or losing units to the
LLM's interpretation.

ADJ21 is the smallest possible step toward that goal: a
**targeted update to `decompose_text`'s prompt** so the LLM
reliably extracts numerical quantities as `quantity(value, unit)`
compound terms. No new primitive. No domain-specific
configuration. The corrected prompt + the IR grammar (already
on main since ADJ01 v3) together cover the contract.

The follow-up — a typed-quantity coverage checker that
validates the LLM did extract every quantity in the source —
is queued as **ADJ22** for a separate PR.

## Why this isn't a new primitive

A natural-looking move would be to add a sibling primitive
called `decompose_typed_facts(text, domain, expected_quantity_types)`.
That's wrong:

- **The IR grammar is already domain-agnostic.** `IRNode` with
  `kind: Fact` and a compound term containing `quantity(value,
  unit)` can express any value-with-unit from any domain. No
  schema extension needed.
- **Pre-declared quantity types couple callers to domains.**
  Asking the caller to list `["blade_length", "volume", "voltage"]`
  for TSA, `["temperature", "heart_rate", "ABV"]` for clinical,
  etc., re-introduces the domain-specific coupling the framework
  is designed to avoid.
- **The model can do this generically with the right prompt.**
  Structured information extraction is a well-studied task. LLMs
  reliably extract `(value, unit)` pairs from arbitrary text when
  the prompt asks for them in a typed shape and shows one worked
  example.

So the fix is in the existing primitive's prompt template, not
in adding a parallel primitive.

## What the prompt change does

Three additions to `decompose_text`'s system prompt:

### 1. A new section: "QUANTITY rules"

Three new rules under their own heading:

- **7a**: Every numerical quantity in the source MUST appear as a
  `quantity(<value>, <unit>)` compound term. Values are atoms
  with literal numbers preserved (no rounding, no conversion).
  Units are snake_case atoms (`oz`, `ml`, `inches`, `wh`, etc.).
  Quantities are embedded inside the surrounding fact's args —
  never flattened into the predicate name.

  > **Wrong**: `blade_4_inches(knife)`
  > **Right**: `blade_length(knife, quantity(4, inches))`

- **7b**: Bare numbers without units are still preserved as
  atoms, using `count` or domain-appropriate predicates.
  `"1 carry-on bag"` → `carry_on_bag(quantity(1, count))`.

- **7c**: The rationale — downstream rules compare quantities
  against thresholds (`gt(L, quantity(2.36, inches))`,
  `le(V, quantity(100, ml))`). The engine evaluates these
  comparisons deterministically — but only if the source IR
  preserves the typed value.

### 2. A second worked example

The existing prompt's worked example uses `"1 carry-on bag,
matches."` — a case without typed quantities. The v5 prompt adds
a second worked example with a numerical quantity:

```
SOURCE: "4 inch pocket knife." (20 bytes)

Output (excerpt):
  N1: declared(pocket_knife)
  N2: blade_length(pocket_knife, quantity(4, inches))
  S1: sentence (covers ". " connective + trailing punctuation)

Coverage tiles 0..20.
```

The example follows the existing prompt conventions (id naming,
source_spans tiling, Section/Contains structure). The only new
shape it teaches is the nested `quantity(4, inches)` compound.

The rationale paragraph after the example connects the typed
quantity to the engine's job:

> *"A downstream rule like `prohibited(X) :- blade_length(X,
> quantity(L, inches)), gt(L, quantity(2.36, inches))` can
> pattern-match `L = 4` and evaluate `4 > 2.36` deterministically
> in the engine. If the IR had flattened the number into the
> predicate name (`blade_4_inches`) or dropped the unit
> (`blade_length(pocket_knife, 4)`), the rule could not fire
> correctly. Preserve the typed value."*

### 3. Prompt version bump

`DECOMPOSE_TEXT_PROMPT_VERSION: "decompose-text-v4" → "decompose-text-v5"`.

The audit-trail discipline requires every prompt change to bump
the version constant so replayed adjudications can distinguish
v4-era IR from v5-era IR. Pre-existing audit records remain
replayable; new records use v5.

## What the prompt change does NOT do

- **No domain-specific unit lists.** The unit examples in the
  rule (`oz`, `ml`, `inches`, `wh`, `celsius`, `percent_abv`,
  etc.) are illustrative, not exhaustive. The model is expected
  to use any sensible snake_case unit atom appropriate to the
  source.
- **No new edge relations.** The existing edge taxonomy
  (`Contains`, `Refers`, `Mentions`, etc.) is sufficient.
  Quantities live INSIDE the surrounding fact's compound term;
  they don't need their own edge relation.
- **No structural changes to the IR grammar.** `IRDocument`,
  `IRNode`, `IREdge` are unchanged. Only the prompt asks for a
  particular shape of compound term inside `IRNode.term`.
- **No new primitive.** `decompose_text` remains the only
  text-to-IR primitive.

## Tests

Three new offline unit tests added to
`adjudication-rust/llm-primitives/src/decompose_text.rs`:

- `system_prompt_documents_typed_quantity_extraction` — pins
  the prompt mentions the quantity-extraction rule, the
  wrong-vs-right pattern (`blade_4_inches` vs
  `blade_length(knife, quantity(4, inches))`), and the common
  unit list.
- `system_prompt_includes_pocket_knife_worked_example` — pins
  the second worked example is present, produces the right shape,
  and the rationale paragraph mentions `4 > 2.36` (the engine's
  evaluation).
- `system_prompt_uses_domain_neutral_quantity_rules` — pins the
  generic phrasing: "Every numerical quantity in the source"
  (not "Every TSA quantity"), `snake_case atom` for units.

Plus the existing `prompt_version_constants_are_stable` test was
updated to expect `decompose-text-v5`.

All 81 lib tests pass.

## How this composes with other ADJs

```
ADJ01 v3 (IR grammar)
   ↓ supports
quantity(value, unit) as a compound term
   ↓ now produced by
decompose_text v5 (this PR)
   ↓ enables
ADJ20 fact sheets to reference typed quantities by entity
   ↓ enables
adjudication-pipeline run_with_rulebooks to lower typed-quantity
rules (rulebook says `gt(L, quantity(2.36, inches))`) and the
engine to unify them with source IR's quantity atoms
   ↓ produces
deterministic verdicts where the LLM never had to do the
arithmetic.
```

The framework's pipeline becomes:

1. **Source decomposition (this PR)** — LLM produces typed
   IR including `quantity(...)` atoms for every numerical literal.
2. **Fact sheets (ADJ20-impl, not yet on main)** — per-entity
   world-knowledge facts that map entity types to applicable
   rule classes.
3. **Rulebook** — domain rules referencing typed quantities and
   thresholds.
4. **Engine** — unifies source IR + fact sheets + rulebook,
   evaluates threshold comparisons deterministically.
5. **Verdict** — proof DAG citing typed-quantity facts, fact-sheet
   entries, and rule clauses by provenance.

The LLM's job shrinks to: produce well-typed IR. The arithmetic
is the engine's.

## ADJ22 (queued): typed-quantity coverage checker

The next sibling spec adds a coverage-checker pass that walks
the source text for numerical literals (regex
`[0-9]+(\.[0-9]+)?\s*\S+`) and verifies each one has a
corresponding `quantity(...)` compound somewhere in the IR with
an overlapping source span. Failures route to ADJ06
clarification — same loop the framework uses for ADJ02 coverage
failures.

ADJ22 is a separate PR because:

- It's a new crate (or extension to `adjudication-coverage`).
- It needs its own prompt-version-aware retry logic.
- It needs its own test suite covering edge cases (numbers in
  compound words, e.g., "30-day window"; ranges, e.g., "5-7
  inches"; ordinals; etc.).

ADJ21 (this PR) lands the prompt change first so it's available
in main; ADJ22 follows with the validator. Both are needed for
the engine arm to work end-to-end on non-canonical declarations.

## Reproduction

The prompt change is observable in the prompt-version constant:

```bash
# After this PR lands:
cargo run -p adjudication-tsa-demo -- ...   # ↑ uses decompose-text-v5
```

Every `LlmCallRecord` for `decompose_text` calls after this PR
will record `prompt_version: "decompose-text-v5"`. Replay of
pre-existing v4-era audit records is unaffected — the framework's
audit trail discipline keeps versions distinguishable.

A second-pass bench (re-running ADJ18 / ADJ19 against v5
decompose_text + Arm B / Arm C) is the natural empirical
follow-up. Queued as ADJ23.

## See also

- [ADJ01](ADJ01-adjudication-ir-grammar.md) — the IR grammar
  that already supports typed quantities; this PR just makes
  the prompt reliably produce them.
- [ADJ18 §"v0.13 re-run results"](ADJ18-broadened-tsa-empirical-bench.md)
  — the empirical evidence that prompt-level ESCALATE is
  insufficient because the LLM cannot evaluate thresholds.
- [ADJ20](ADJ20-fact-sheets-and-pipeline-reorder.md) — the
  fact-sheet primitive that consumes typed quantities from
  source IR + rules from rulebooks.
- ADJ22 (queued) — typed-quantity coverage checker.
- ADJ23 (queued) — empirical re-bench post-v5 decompose_text.
