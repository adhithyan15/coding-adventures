# ADJ27 — Content-Shaped Decomposition: Empirical Results

> Follow-up to [ADJ26](ADJ26-foundation-bench.md). After the v1
> foundation bench produced 0/40 usable IRs, two structural changes
> landed in `decompose_level`:
>
> 1. **Per-level prompts** ([#3130](https://github.com/adhithyan15/coding-adventures/pull/3130)):
>    one focused system prompt per decomposition level, replacing the
>    monolithic `decompose-text-v5`.
> 2. **Content-shaped contract** (this PR): the LLM emits the
>    LITERAL substring it claims per child (`text` field), and the
>    framework matches that text against the parent text to derive
>    byte spans. The model never does byte arithmetic.
>
> Re-ran the ADJ26 8 × 5 matrix against the combined changes on
> 2026-05-13. **3/40 cells fully passed** (vs 0/40 in v1) — but the
> data is nuanced, and the most interesting findings are about
> *partial progress*, not the headline pass rate. Raw data:
> [`data/adj27-foundation-bench-2026-05-13-content-shaped.json`](data/adj27-foundation-bench-2026-05-13-content-shaped.json).

## Headline numbers

| Metric | v1 (byte-shaped) | v2 (content-shaped) |
|---|---|---|
| Cells fully passing | 0 / 40 | **3 / 40** |
| Cells producing any IR (made it past the orchestrator) | 0 / 40 | 40 / 40 |
| Total wallclock | 36.8 min | 47.5 min |
| Cells that retried at the coverage-loop stage | 0 / 40 | **37 / 40** |

The "made it past the orchestrator" axis is the real story. In v1,
every cell failed before the orchestrator could even check coverage —
the model produced flat-IR kinds the per-level filter rejected. In
v2, every cell produces hierarchy-shaped children the orchestrator
splices into the IR; the failure mode shifted from "no IR at all"
to "coverage gaps that the retry budget couldn't close."

## The per-cell grid

Cell value = `PASS` (fully clean) / `g=N` (coverage failure with N
gaps after retries) / `unparse` (model returned invalid JSON):

| Declaration | gemma4:latest | llama3.1:8b | qwen2.5:3b | qwen2.5:1.5b | qwen2.5:0.5b |
|---|---|---|---|---|---|
| matches | g=3 | g=7 | unparse | **g=1** | g=2 |
| large-lithium | **PASS** | g=11 | g=1 | **g=1** | g=4 |
| large-toothpaste | g=11 | g=4 | g=10 | g=5 | g=2 |
| pocket-knife | g=11 | g=8 | unparse | **g=1** | g=4 |
| wine-bottle | **PASS** | g=8 | g=1 | **g=1** | g=5 |
| small-lithium | **PASS** | g=12 | g=1 | g=5 | g=5 |
| small-perfume | g=9 | g=8 | g=9 | g=5 | g=3 |
| lighter-disposable | g=10 | g=8 | unparse | g=3 | g=3 |

## Finding 1 — Smaller models got closer to passing

Average gap counts among cells that hit the coverage-retry stage:

| Model | Avg gaps | Range | Cells g≤2 |
|---|---|---|---|
| gemma4:latest | 8.8 | 3 – 11 | 0 |
| llama3.1:8b | 8.2 | 4 – 12 | 0 |
| qwen2.5:3b | 4.4 | 1 – 10 | 4 |
| **qwen2.5:1.5b** | **2.8** | **1 – 5** | **3** |
| qwen2.5:0.5b | 3.5 | 2 – 5 | 2 |

**qwen2.5:1.5b had the lowest average gap count** and the most cells
within one retry of passing (3 cells at g=1 — `matches`,
`large-lithium`, `pocket-knife`). The 8B reference models
(gemma4, llama3.1) had the *highest* gap counts.

**Hypothesis**: bigger models decompose more aggressively at every
level, producing more children per parent. More children = more
parent-level coverage boundaries that can fail. Smaller models
produce coarser-grained children (often 1-2 per parent), which
trivially tile.

This contradicts the v1 prediction in
[ADJ26 §H2](ADJ26-foundation-bench.md#hypotheses) ("Per-parent
fresh-agent retries with parent-scoped prompts work for the 1.5B and
0.5B models in ways that the prior whole-document retries did not")
— but flips the direction: small models do *better* at partial
coverage, not just *as well as* the larger ones. The framework's
"shrink the model down" thesis gets concrete support from this
asymmetry.

## Finding 2 — The "Discarded escape hatch" is a real bug

All 3 fully-passing cells (`large-lithium`, `wine-bottle`,
`small-lithium` × gemma4) used a degenerate strategy: the model
emitted a single `Discarded` node covering the entire document at
level 1, satisfying byte coverage trivially. Subsequent levels had
no parents to decompose (Discarded nodes don't have children), and
the orchestrator's per-level check passed every boundary by virtue
of having no boundary to check.

Each of these "passing" IRs had only 2–5 nodes total with
`kinds_present: ['Discarded', 'Document']`. **No actual semantic
content was extracted.** They pass the bench because the bench
treats coverage as the only gate; they shouldn't count as
production-quality decompositions.

The fix is structural: per-level system prompts (the new
`decompose-level-v1` family) need to discourage Discarded for whole
parents. Discarded should be the exception for chunks that genuinely
don't belong (document metadata, page numbers, salutations) — not
the lazy escape from doing the decomposition work.

Concrete prompt-level interventions to test in the next iteration:

1. Add to each level's prompt: *"You may only mark a child as
   Discarded if it covers a SUBSET of the parent — never the whole
   parent. At least one non-Discarded child is required."*
2. Make `discard_reason` more discriminating — currently the prompt
   accepts vague reasons like `NonDomainContent`. Tighter constraint:
   each Discarded must cite a specific reason from a closed set with
   ban on "doesn't fit / not domain content" for whole-document
   discards.
3. Run a Doc → Sentence-only pass first with strict no-Discarded;
   later levels can opt back in.

## Finding 3 — qwen2.5:3b's parse failures at level 4

3 of 8 qwen2.5:3b cells failed with `unparseable_at_FactToTypedComponent`
— the model couldn't produce parseable JSON at the typed-component
level. The typed-component prompt is the longest of the four
(2,277 chars), with 7 kind options and the no-flattening rule.

The 1.5B and 0.5B models did *not* hit this failure mode
(0 unparseable cells). They produced parseable JSON but with
coverage gaps. qwen2.5:3b appears to be the unstable size — large
enough to attempt the structure, not large enough to consistently
emit valid JSON for the complex prompt.

Either:
- Compress the TYPED_COMPONENT_PROMPT (currently ~2.3KB; could be
  ~1.5KB by trimming kind explanations).
- Drop `format: json` for this model and parse defensively.
- Use a different model at this level.

## Finding 4 — Wallclock cost

The content-shaped contract is ~30% slower wallclock than the
byte-shaped baseline (47.5 min vs 36.8 min for 40 cells), because:

- Per-cell average ~7s on the smallest model, ~140s on the largest.
- Retries fire on more cells (37 of 40 hit the coverage-retry stage
  vs 0 in v1).
- Each retry is one more LLM call with growing prior-attempt JSON
  in the prompt.

Acceptable for the headline improvement (3 passing + 37 with
partial IR vs 0 anything). The path to faster bench iteration is
*reducing retry count*, not *reducing per-call cost* — once the
prompts converge enough that most cells pass on the first or
second attempt, total wallclock drops.

## What this validates (and doesn't)

**Validated:**

- **The framework's audit shape works under real load**: every
  failure was a typed orchestrator error with structured gap data.
  No silent confabulation. The pipeline correctly identifies "model
  said X, parent has Y, the missing piece is `Z`."
- **Pulling byte arithmetic into the framework was the right call**:
  v1's 0/40 wasn't a "models are bad at hierarchy" result — it was a
  "models are bad at byte offsets" result. With offsets out of the
  way, hierarchy + content come through.
- **Per-level prompts are correct shape**: 7 of 8 cells reached the
  retry stage at level 4 (Fact → TypedComponent), meaning levels 1–3
  produced parseable hierarchical output across the lineup.

**Not validated / still open:**

- **The Discarded escape hatch** makes the 3 "passing" cells not
  count semantically. Fixing requires another prompt iteration.
- **No model produces consistent fully-passing IR**. qwen2.5:1.5b is
  closest (3 of 8 cells at g=1) but still doesn't close those gaps
  in the current retry budget.
- **qwen2.5:3b's JSON instability** suggests we may need
  model-specific tuning at level 4, or a different way to coax the
  output schema.

## Gating condition — still NOT met

The unblock gate from ADJ25 is "reliable per-level coverage across
the 8 × 5 matrix." 3 degenerate passes + many partial-coverage
attempts don't meet any reasonable threshold. **The paused
workstreams stay paused.**

## Next interventions, in order

1. **Patch the Discarded escape hatch**: add the "no whole-parent
   Discarded" constraint to all four per-level prompts. Re-bench.
2. **Compress TYPED_COMPONENT_PROMPT** for qwen2.5:3b stability.
3. **Bump per-parent retry budget from 3 to 5–8** — qwen2.5:1.5b's
   g=1 cells likely close with 1-2 more retries given the right gap
   description.
4. **Investigate the level-by-level call latency on gemma4** —
   140s avg per cell is concerning given the small-input shape.
   Might be Ollama-side model swap overhead, not generation.

After 1+2+3, re-run the bench. If small models hit ≥ 50% fully
passing, the gating condition is in reach.

## See also

- [ADJ25](ADJ25-hierarchical-decomposition.md) — the spec the
  contract evolution serves.
- [ADJ26](ADJ26-foundation-bench.md) — methodology + v1 bench
  results.
- [`feedback_no_byte_arithmetic_for_llm`](../../memory/feedback_no_byte_arithmetic_for_llm.md)
  — the design lesson the content-shaped contract codifies.
- [`feedback_adjudication_per_level_prompts`](../../memory/feedback_adjudication_per_level_prompts.md)
  — the "small focused prompts" thesis.

## Status

- 2026-05-13: content-shaped contract landed in the merged
  [#3130](https://github.com/adhithyan15/coding-adventures/pull/3130)
  (per-level prompts) + this PR's follow-up commit.
- Bench results captured. Discarded escape hatch identified as the
  next target.
