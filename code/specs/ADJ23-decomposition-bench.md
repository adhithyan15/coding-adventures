# ADJ23 — Decomposition bench (typed-quantity recall)

## Overview

ADJ23 is the empirical counterpart to
[ADJ21](ADJ21-typed-quantity-decomposition.md) (prompt change)
and [ADJ22](ADJ22-typed-quantity-coverage.md) (validator).
Where ADJ18 measured *verdict-side* accuracy of the raw-LLM Arm A,
ADJ23 measures **decomposition-side** behaviour: when the v5
prompt asks the LLM to emit `quantity(value, unit)` compounds for
every numerical literal, how often does the LLM actually do so?

## What this bench measures

For each cell `(declaration, model)`:

1. Drive `adjudication-tsa-demo` with `ADJ_DEMO_IR_MODE=llm` and
   `ADJ_DEMO_AUDIT=1`. This forces the demo's Arm B to call
   `decompose_text` (the prompt updated by ADJ21) and dumps the
   full audit trail to stdout.
2. Parse the audit-trail JSON, walk the IR term tree, and collect
   every `quantity(<value>, <unit>)` compound.
3. For each numerical literal in the source declaration (regex
   `\d+(\.\d+)?`, same as ADJ22), check whether a matching
   quantity compound was extracted (after the same
   normalisation: `"4"`, `"4.0"`, `"04"` all canonicalise to `"4"`).

Two headline metrics:

- **Typed-quantity recall** — `matched_literals / total_literals`
  across the matrix. The fraction of numerical literals that
  survived as typed quantities.
- **ADJ22 pass rate** — fraction of cells where every literal in
  the source got a matching quantity in the IR (i.e. the IR
  would pass the ADJ22 validator first-try).

Arm A verdict is captured for completeness but is not the
headline — that's ADJ18's measurement. ADJ23 is about whether
the *decomposition contract* (ADJ21) is honoured in practice.

## The matrix

Same shape as ADJ18: 8 declarations × 5 models = 40 cells.

Declarations (and the literals each contains):

| id                | text                                           | literals     |
|-------------------|------------------------------------------------|--------------|
| matches           | `1 carry-on bag, matches.`                     | `1`          |
| large-lithium     | `1 carry-on bag, lithium battery, 200 Wh.`     | `1`, `200`   |
| large-toothpaste  | `1 carry-on bag, 4 oz toothpaste.`             | `1`, `4`     |
| pocket-knife      | `1 carry-on bag, 4 inch pocket knife.`         | `1`, `4`    |
| wine-bottle       | `1 carry-on bag, 1 bottle of wine, 750 ml.`    | `1`,`1`,`750`|
| small-lithium     | `1 carry-on bag, lithium battery, 50 Wh.`      | `1`, `50`    |
| small-perfume     | `1 carry-on bag, 3 oz perfume.`                | `1`, `3`     |
| lighter-disposable| `1 carry-on bag, disposable lighter.`          | `1`          |

Models: `gemma4:latest`, `llama3.1:8b`, `qwen2.5:3b`,
`qwen2.5:1.5b`, `qwen2.5:0.5b`.

Cell timeout: 15 min (decompose_text + ADJ04 + ADJ05 chained
LLM calls per cell — meaningfully longer than ADJ18's Arm A only).

## Harness

`scripts/adj23_decomposition_bench.py`. Same structural idea as
`scripts/adj18_tsa_arm_a_bench.py` but Arm B + audit trail:

- Sets `ADJ_DEMO_IR_MODE=llm` + `ADJ_DEMO_AUDIT=1`.
- Captures stdout, extracts the JSON dumped after
  `--- full audit trail (ADJ07-v1) ---`.
- Walks every dict in the audit trail looking for objects with
  `{"functor": "quantity", "args": [...]}` shape; pulls the first
  arg (canonicalised) as the literal value and the second as the
  unit.
- Canonicalises both the source literals and the IR's extracted
  literals the same way ADJ22 does (split on `.`, strip leading
  zeros from the whole part down to at least one digit, strip
  trailing zeros from the fractional part).
- Writes a JSON result file at
  `code/specs/data/adj23-decomposition-bench-2026-05-13.json`.

## Results

> Bench run: 2026-05-13 (decompose-text-v5, llm-primitives v0.11.0,
> adjudication-coverage v0.3.0). Raw results filed at
> [`code/specs/data/adj23-decomposition-bench-2026-05-13.json`](data/adj23-decomposition-bench-2026-05-13.json).

**HEADLINE METRICS**:

- Total cells: **40** / 40.
- ADJ22 pass: **4 / 40 (10.0%)**.
- Typed-quantity recall: **21 / 75 (28.0%)**.

The flat numbers hide a sharply bimodal distribution: gemma4
nails the contract when it can emit valid JSON; llama3.1:8b
emits units but skips counts; the three qwen2.5 sizes essentially
ignore the v5 prompt. Details follow.

### Per-model breakdown

| Model            | ADJ22 pass | Recall      | Median wallclock |
|------------------|-----------:|------------:|-----------------:|
| gemma4:latest    | 4/8 (50%)  | 9/15 (60%)  | 199.7s           |
| llama3.1:8b      | 0/8 ( 0%)  | 6/15 (40%)  |  70.1s           |
| qwen2.5:3b       | 0/8 ( 0%)  | 2/15 (13%)  |  35.0s           |
| qwen2.5:1.5b     | 0/8 ( 0%)  | 2/15 (13%)  |  26.9s           |
| qwen2.5:0.5b     | 0/8 ( 0%)  | 2/15 (13%)  |  12.9s           |

### Per-declaration breakdown

| Declaration        | ADJ22 pass | Recall      |
|--------------------|-----------:|------------:|
| large-lithium      | 1/5 (20%)  | 4/10 (40%)  |
| large-toothpaste   | 1/5 (20%)  | 4/10 (40%)  |
| pocket-knife       | 0/5 ( 0%)  | 3/10 (30%)  |
| wine-bottle        | 1/5 (20%)  | 5/15 (33%)  |
| small-lithium      | 1/5 (20%)  | 3/10 (30%)  |
| small-perfume      | 0/5 ( 0%)  | 2/10 (20%)  |
| matches            | 0/5 ( 0%)  | 0/5  ( 0%)  |
| lighter-disposable | 0/5 ( 0%)  | 0/5  ( 0%)  |

### Notable patterns

**1. The "1 carry-on bag" count quantity is almost never extracted.**
Across every cell, the source literal `1` for the bag count failed
to surface as a typed quantity in **37/40 cells (92.5%)**. The
three cells where it succeeded were exactly gemma4 on its three
"easy" decls (large-lithium, large-toothpaste, small-lithium —
which have a second numerical literal that primes the schema).
Even gemma4 dropped the count on declarations with no other
number (matches, lighter-disposable). Models treat the count as
schema decoration ("there's one declaration"), not as a
measurement worth typing.

**2. gemma4's 50% ADJ22 pass rate is *bimodal*: when it works,
it works perfectly; when it fails, decompose_text fails outright.**
On 4/8 cells gemma4 returned malformed JSON (truncated mid-string
or mid-object — the largest at line 83 col 20702, ~20 KB of
output before EOF). The demo falls back to a hand-built fixture,
which has no typed quantities by construction. On the other 4/8
cells gemma4 produced perfect IR — every numerical literal got a
`quantity(value, unit)` compound. There's no "tried but missed"
middle ground for this model on this dataset.

| gemma4 result | cells | typical literal in IR                |
|---------------|------:|--------------------------------------|
| ADJ22 pass    |   4   | `quantity(200, wh)`, `quantity(1, count)` |
| JSON failure  |   4   | (fell back to hand-built IR)         |

**3. llama3.1:8b reliably extracts units but skips counts.**
Across 8 cells llama3.1 emitted `quantity(<n>, oz)`,
`quantity(<n>, ml)`, `quantity(<n>, wh)`,
`quantity(<n>, inch(es))` correctly for every *measurement*
literal — but it never produced `quantity(1, count)` for the bag.
That's the 6/15 recall: 5 measurement literals captured, 7 count
literals dropped, plus 3 lost to the lighter/matches/wine-second-1
cases.

**4. qwen2.5 sizes (3b, 1.5b, 0.5b) essentially ignore the v5
prompt's quantity rule.** Recall hovers at 13% across all three
sizes — they emit numbers as bare atoms inside domain predicates
(`battery_capacity(50_wh)` or `volume(750ml)`) or skip them
entirely. A handful of cells emitted the right value with the
unit garbled (`quantity(750, ?)` from qwen2.5:1.5b on wine-bottle
where the unit position was dropped) — recorded as a recall hit
since ADJ22 v0.1 doesn't enforce unit atoms, but those would fail
a future per-domain unit-vocabulary check.

**5. The pocket-knife declaration — the original motivating
case — has 0/5 ADJ22 pass.** Three smaller models extracted the
critical blade-length quantity (`quantity(4, inches)`) but missed
the bag count. gemma4 truncated the JSON entirely. So the
end-to-end typed-quantity story for the pocket-knife regression
needs both pieces to work: the model emits the right quantity AND
the rest of the IR is valid enough to ship.

## Reading the results

The check is **structural**, not semantic. A cell passes ADJ22 if
*any* `quantity(<lit>, _)` compound exists in the audit-trail IR
for each source literal. We don't check whether the unit is
right, the value is meaningful, or the surrounding predicate is
sane. ADJ23 measures only whether the LLM honoured the
*shape* of the contract.

That's deliberate. The downstream wiring (ADJ22 → ADJ06
clarification → re-extract) handles the case where the model
flubs the shape: the clarification prompt names the missing
literal and the model gets a second pass. ADJ23 measures
first-pass behaviour, the upper bound on how well the simple
prompt change works without any retry support.

## What this unlocks

10% first-pass ADJ22 is unshippable on its own — but the
breakdown points to *three* specific, separate follow-up workstreams,
each with a clean fix:

**Workstream A — ADJ22 → ADJ06 retry-loop wiring (high priority).**
The dominant failure mode (count-quantity dropped on the bag)
is exactly the case ADJ06 clarification was designed for:
ADJ22 spots the missing literal, ADJ06 re-prompts with
*"You produced N1 over the range '1 carry-on bag' but its term
did not include `quantity(1, _)`. Please re-extract."* The
prompt-template plus the wiring change is plausibly enough to
move llama3.1's recall from 40% to ≥80% without touching the
v5 prompt. Tracked as ADJ24.

**Workstream B — gemma4 JSON-emission cap (medium priority).**
4/8 gemma4 cells truncated mid-JSON at output sizes well under
its theoretical context window. The fix isn't on the prompt
side; it's a gateway-level investigation:
- Is `ADJ_DEMO_MAX_ANSWER_TOKENS=2048` insufficient for gemma4's
  verbose IR shape (gemma4 hit ~14 KB before EOF in one case)?
- Does ollama need a different sampling parameter for long
  structured output (lower `top_p`, higher `repeat_penalty`)?
- Should `decompose_text` self-retry with a higher token budget
  when it sees an unterminated JSON error?
Until this is fixed, gemma4 is the wrong upper-bound: its 50%
ADJ22 pass is "ceiling among produced output", not "ceiling
overall". Worth pursuing because gemma4's *content* on
successful cells is excellent (every count + measurement typed,
zero flattening into predicate names).

**Workstream C — qwen2.5 family (low priority for typed
quantities specifically).** ADJ12/ADJ18 already documented that
qwen2.5:1.5b/0.5b ignore most rule-following prompts; ADJ23
confirms the v5 quantity rule is no exception. The structural
fix for very-small models is probably grammar-constrained decoding
of the `quantity(...)` compound rather than prompt engineering.
Defer until the bigger models are clean.

**Cross-cutting follow-up: don't rely on the LLM for counts.**
The "1 carry-on bag" pattern suggests a domain-level fact-sheet
approach: treat declaration cardinality as a deterministic field
on the declaration record, not something the model has to
hand-extract from text. ADJ20 (fact sheets + pipeline reorder)
is the right home for that.

## See also

- [ADJ18](ADJ18-broadened-tsa-empirical-bench.md) — Arm A verdict
  bench that motivated ADJ21 + ADJ22.
- [ADJ21](ADJ21-typed-quantity-decomposition.md) — the v5
  decompose prompt contract.
- [ADJ22](ADJ22-typed-quantity-coverage.md) — the validator that
  enforces the same contract pre-engine.
- ADJ24 (queued) — pipeline wiring for ADJ22 → ADJ06 with a
  follow-up bench showing the retry-loop improvement.
