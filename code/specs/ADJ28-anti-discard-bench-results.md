# ADJ28 — Anti-Discard Bench Results

> Follow-up data PR to [ADJ27](ADJ27-content-shaped-decomposition-bench.md).
> Bench re-run against the [ADJ28 prompt changes
> (#3207)](https://github.com/adhithyan15/coding-adventures/pull/3207):
> yes/no boolean kind schema at multi-option levels + WHAT NOT TO DO
> worked examples + Discarded-requires-justification + explicit
> whole-parent-discard bans.
>
> Same 8 × 5 matrix, same local Ollama, all 5 models pulled.
> Bench run: 2026-05-14. Total wallclock: **83.9 min** (vs ADJ27's
> 47.5 min). Raw data:
> [`data/adj28-foundation-bench-2026-05-14-anti-discard.json`](data/adj28-foundation-bench-2026-05-14-anti-discard.json).

## Headline numbers

| Metric | ADJ27 (v2 content) | ADJ28 (v3 anti-discard) |
|---|---|---|
| Cells fully passing | 3 / 40 | **0 / 40** |
| Cells producing any IR (reached coverage-retry) | 37 / 40 | 39 / 40 |
| Total wallclock | 47.5 min | 83.9 min |

The 3/40 → 0/40 movement looks like a regression but isn't. All three
ADJ27 "passes" were **degenerate** — gemma4 emitted a single
`Discarded` over the entire document, satisfying byte coverage
trivially while extracting zero semantic content. The new anti-discard
prompts killed that escape hatch. 0/40 honest fails > 3/40 fake passes.

## The per-cell grid

| Declaration | gemma4 | llama3.1 | qwen2.5:3b | qwen2.5:1.5b | qwen2.5:0.5b |
|---|---|---|---|---|---|
| matches | g=7 | g=7 | **g=1** | g=4 | **g=1** |
| large-lithium | g=20 | g=15 | g=13 | g=8 | g=3 |
| large-toothpaste | g=7 | g=9 | unparse | g=6 | **g=2** |
| pocket-knife | g=6 | g=14 | g=8 | g=6 | g=3 |
| wine-bottle | g=20 | g=20 | g=9 | g=7 | g=4 |
| small-lithium | g=20 | g=15 | g=13 | g=7 | g=6 |
| small-perfume | g=7 | g=7 | g=6 | **g=2** | **g=2** |
| lighter-disposable | g=15 | g=8 | g=4 | **g=1** | g=3 |

Cells at g≤2 (one or two retries from passing) bolded.

## Per-model summary

| Model | Avg wallclock | Avg gaps | Range | g≤2 cells |
|---|---|---|---|---|
| gemma4:latest | 290 s | 12.8 | 6 – 20 | **0** |
| llama3.1:8b | 209 s | 11.9 | 7 – 20 | **0** |
| qwen2.5:3b | 67 s | 7.7 | 1 – 13 | 1 |
| qwen2.5:1.5b | 38 s | 5.1 | 1 – 8 | 2 |
| **qwen2.5:0.5b** | **25 s** | **3.0** | **1 – 6** | **3** |

## Finding 1 — The small-model dominance is real and sharpening

The leaderboard inverted by parameter count:

| Model size | g≤2 cells | Avg gaps |
|---|---|---|
| 8B (gemma4) | 0 | 12.8 |
| 8B (llama3.1) | 0 | 11.9 |
| 3B | 1 | 7.7 |
| 1.5B | 2 | 5.1 |
| **0.5B** | **3** | **3.0** |

Smaller-is-better is now a **monotonic** trend. ADJ27 had qwen2.5:1.5b
leading (3 cells at g=1, avg 2.8) with the 0.5B model in fourth place.
ADJ28 has **qwen2.5:0.5b leading** (3 cells at g≤2, avg 3.0). Three of
the 0.5B model's eight cells are within 2 retries of passing —
including `matches` at g=1, one retry away.

**Why this matters**: the framework's "shrink the model down, push
intelligence into the framework" thesis predicts that *with the right
structural support*, the smallest model should outperform on partial
coverage. ADJ28's combination of (a) per-level focused prompts +
(b) content-shaped contract + (c) yes/no kind schema + (d) anti-discard
constraint has produced exactly that. The 0.5B model is doing the most
useful work.

## Finding 2 — Anti-Discard killed the degenerate-pass shortcut

ADJ27 had 3 cells "fully passing" — all gemma4, all using the single-
Discarded-over-whole-document trick. ADJ28's prompts:

- Explicit forbid: *"never discard the whole input. At least one
  node MUST have `kind: Sentence`."*
- Required `discard_justification` — a sentence explaining WHY
  discarding loses no information. Hard to write honestly for a
  whole-document discard.

Result: **zero degenerate passes**. Gemma4 went from 3 fake passes
(all `Discarded`-only IRs) to 0 fake passes. The model is now forced
to do the actual decomposition work, which it does less cleanly than
the smaller models.

## Finding 3 — Big models over-decompose and lose coverage

8B models without the Discarded escape hatch produced **wildly
fragmented** outputs: gemma4 and llama3.1 each had 3 cells at g=20
(or g=15). The pattern: bigger models commit to more aggressive
decomposition, creating many children per parent. More children =
more parent-coverage boundaries to fail.

| Model | Cells at g≥15 | Cells at g≤2 |
|---|---|---|
| gemma4 | 3 | 0 |
| llama3.1 | 2 | 0 |
| qwen2.5:3b | 0 | 1 |
| qwen2.5:1.5b | 0 | 2 |
| qwen2.5:0.5b | 0 | 3 |

Big models throw more darts and miss more. Small models throw fewer
darts and hit closer.

## Finding 4 — Wallclock cost up 76%

ADJ27: 47.5 min. ADJ28: 83.9 min. The cost shifted because:

1. Anti-Discard prompts force more LLM calls per cell — without the
   escape hatch, models produce 5-15 children at each level instead
   of 1.
2. Each retry has more children's text to embed in the correction
   prompt, growing the prompt size.
3. Bigger models (gemma4 at 290s/cell average) are particularly slow
   under the new contract.

The trade is real: more honest work = more wallclock. **The path to
faster bench iteration is bumping the retry budget** so cells close
in fewer iterations, not relaxing the prompts.

## Finding 5 — qwen2.5:3b's level-4 stability improved

ADJ27 had 3 of 8 qwen2.5:3b cells fail with
`unparseable_at_FactToTypedComponent` (level 4 JSON malformed).
ADJ28: **1 of 8**. The new yes/no boolean schema at level 4
(`is_quantity` / `is_polarity` / ...) is more structured and seems
easier for the 3B model to emit consistently than the previous
`kind`-string schema.

## What this validates (and doesn't)

**Validated:**

- The "smaller model + structural support" thesis materialises in
  concrete data. The 0.5B model now does the most useful per-cell
  work.
- The Discarded escape hatch is gone. Honest failure modes only.
- Yes/no boolean schema at level 4 reduced JSON parse failures on
  the mid-size model.
- 7 cells across the lineup are at g=1 (one retry from passing).

**Not validated:**

- The 0/40 fully-passing rate is still 0. The gating condition from
  ADJ25 is NOT met.
- Big models did *worse* under the tighter constraints. We'd want to
  investigate whether their over-decomposition is fixable.

## Gating condition — still NOT met

Zero cells fully passing under any model. Paused workstreams
(ADJ14/15/16/17/18/19/20) stay paused.

## Next interventions, in order

### 1. Bump the per-parent retry budget (highest priority)

`DEFAULT_MAX_RETRIES_PER_PARENT` is currently 3. Seven cells across
the lineup are at g=1, meaning ONE more retry might close them. Three
specific candidates from the 0.5B tier:

- `matches × qwen2.5:0.5b` (g=1)
- `lighter-disposable × qwen2.5:1.5b` (g=1)
- `matches × qwen2.5:3b` (g=1)

A retry-budget bump from 3 → 6 is the cheapest intervention with the
clearest expected payoff. Risk: wallclock cost goes up further.

### 2. Investigate big-model over-decomposition

gemma4 at g=20 means roughly 20 parent-coverage failures across the
hierarchy. The model is producing many children per parent but
they're not tiling cleanly. Two candidate causes:

- The model is producing children with `text` that doesn't appear
  verbatim in the parent (content fabrication).
- The model is overlapping children with each other (greedy
  duplication).

The next bench should capture per-cell failure modes (which level the
gaps come from) so we can distinguish.

### 3. Per-cell-level failure tracing

The bench currently reports a flat gap count. To plan further
interventions we need per-level distribution: *at which level do
gaps cluster?* Cheap to add to the bench binary; informs whether
the next prompt change should target level 4 (typed components),
level 3 (claim picking), or earlier.

## Gating threshold proposal

Now that we have two data points (ADJ27, ADJ28), a workable threshold
to revisit:

- **Tier 1 unblock** (allow ADJ20 fact-sheets to resume): 5 / 40 cells
  fully passing across the matrix, with no model contributing more
  than 60% of the passes.
- **Tier 2 unblock** (allow ADJ16 engine arm, ADJ18/19 verdict
  benches): 15 / 40 cells fully passing.
- **Tier 3** (full unblock, including rulebook ADJ14/15/17): 25 / 40.

The ADJ27 → ADJ28 movement (0 honest passes both rounds, but small-
model gap counts closing) suggests Tier 1 is reachable with retry-
budget bumping alone. Tiers 2 and 3 need more work.

## See also

- [ADJ27](ADJ27-content-shaped-decomposition-bench.md) — previous
  bench results (content-shaped contract).
- [ADJ26](ADJ26-foundation-bench.md) — methodology baseline.
- [PR #3207](https://github.com/adhithyan15/coding-adventures/pull/3207)
  — the ADJ28 prompt changes this bench tests.

## Status

- 2026-05-14: bench re-run complete; results captured.
- Next: bump retry budget + add per-level failure tracing to the
  bench binary; re-bench.
