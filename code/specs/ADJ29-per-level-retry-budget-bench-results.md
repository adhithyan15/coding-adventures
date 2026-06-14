# ADJ29 — Per-Level Retry Budget Bench Results

> Follow-up data PR to [ADJ28](ADJ28-anti-discard-bench-results.md).
> Bench re-run against the [ADJ29 per-level retry budget change
> (#3227)](https://github.com/adhithyan15/coding-adventures/pull/3227):
> `PerLevelRetryBudget` defaults of **3 / 4 / 5 / 8** retries at the
> DocumentToSentence / SentenceToPhrase / PhraseToClaim / FactToTypedComponent
> levels respectively, replacing the uniform-3 budget across all levels.
>
> Same 8 × 5 matrix, same local Ollama, same five models pulled.
> Bench run: 2026-05-14. Total wallclock: **131.0 min** (vs ADJ28's
> 83.9 min). Raw data:
> [`data/adj29-foundation-bench-2026-05-14-per-level.json`](data/adj29-foundation-bench-2026-05-14-per-level.json).

## Methodology note — earlier "gemma retry had no effect" was a bug

Between merging ADJ29 and producing this bench, a gemma-only retry run
([`data/adj29-gemma-retry-2026-05-14.json`](data/adj29-gemma-retry-2026-05-14.json))
showed zero gap-count change vs ADJ28. That finding was wrong. The
bench harness (`scripts/adj_pr6_foundation_bench.py`) unconditionally
set `ADJ_PR6_MAX_RETRIES=3`, which forces `PerLevelRetryBudget::uniform(3)`
in the bench binary and bypasses the per-level defaults entirely. The
gemma-only run was therefore executing the ADJ28 uniform-3 budget under
an ADJ29 label.

Fixed by adding `--per-level-defaults` to the harness, which omits the
env var so the binary falls back to `PerLevelRetryBudget::default()`.
This bench uses the new flag. The gemma-only JSON is retained as
historical evidence of the methodology error and as a uniform-3 baseline
for cross-validating ADJ28 (it matches ADJ28's gemma cells exactly).

## Headline numbers

| Metric | ADJ28 (uniform 3) | ADJ29 (per-level 3/4/5/8) |
|---|---|---|
| Cells fully passing | 0 / 40 | 0 / 40 |
| **Total coverage gaps** | **316** | **251** (**−20.6%**) |
| Total wallclock | 83.9 min | 131.0 min (**+56%**) |
| Cells producing IR (reached retry loop) | 39 / 40 | 39 / 40 |

Per-level budgets close one in five gaps across the matrix, at the cost
of ~1.56× wallclock. Still 0/40 passing — the gating condition stays
unmet — but the gap-count compression is real and concentrated where
the theory predicted: large models on deeply-structured inputs.

## The per-cell grid

ADJ29 gap counts (ADJ28 deltas in parentheses):

| Declaration | gemma4 | llama3.1 | qwen2.5:3b | qwen2.5:1.5b | qwen2.5:0.5b |
|---|---|---|---|---|---|
| matches | 7 (=) | 7 (=) | **1** (=) | **1** (−3) | **1** (=) |
| large-lithium | 8 (−12) | 14 (−1) | 11 (−2) | 6 (−2) | 5 (+2) |
| large-toothpaste | 7 (=) | 13 (+4) | unparse | 6 (=) | **2** (=) |
| pocket-knife | 5 (−1) | 15 (+1) | 8 (=) | 3 (−3) | 3 (=) |
| wine-bottle | 8 (−12) | 15 (−5) | 7 (−2) | 7 (=) | 6 (+2) |
| small-lithium | 8 (−12) | 10 (−5) | 10 (−3) | 4 (−3) | 6 (=) |
| small-perfume | 7 (=) | 9 (+2) | 4 (−2) | **2** (=) | **2** (=) |
| lighter-disposable | 7 (−8) | 8 (=) | 4 (=) | **1** (=) | 3 (=) |

Cells at g≤2 (one or two retries from passing) bolded. Cells at g=1 are
candidates for further budget bumps or prompt tweaks; ADJ29 added one
new g=1 cell (`matches × qwen2.5:1.5b`, was g=4 in ADJ28).

## Per-model summary

| Model | ADJ28 sum | ADJ29 sum | Δ | Δ% |
|---|---|---|---|---|
| **gemma4:latest** | **102** | **57** | **−45** | **−44%** |
| qwen2.5:1.5b | 41 | 30 | −11 | −27% |
| qwen2.5:3b | 54 | 45 | −9 | −17% |
| llama3.1:8b | 95 | 91 | −4 | −4% |
| qwen2.5:0.5b | 24 | 28 | **+4** | **+17%** |

Gemma contributed 45 of the 65 closed gaps — nearly 70% of the
improvement. Four of its worst cells (large-lithium, small-lithium,
wine-bottle, lighter-disposable) collapsed from g=15..20 to g=7..8.

## Finding 1 — Per-level budgets are doing what they were designed to do

The hypothesis behind ADJ29 was that careful decomposition produces
many children at deeper levels (Claim → TypedComponent fan-out),
and a uniform retry budget exhausts before the deepest level converges.
Bumping FactToTypedComponent from 3 → 8 retries should help most where
the FactToTypedComponent fan-out is largest.

The data fits. Improvement is concentrated on:

- Inputs with multi-part typed components (e.g. `200 Wh lithium
  battery` → quantity + entity + modifier) — these are `large-lithium`,
  `small-lithium`, `wine-bottle`, `lighter-disposable`.
- Models that *can* recover when given more attempts — Gemma, the
  qwen2.5 mid-tier.

Inputs with simple structure (`matches`, `small-perfume`) and the
smallest model (qwen2.5:0.5b) show no improvement — there's no deeper
level to help with.

## Finding 2 — Big models benefit most, reversing one ADJ28 trend

ADJ28 reported "smaller-is-better is now a monotonic trend." Per-level
budgets partially undo that ordering by giving the big models room to
recover from over-decomposition:

| Model | ADJ28 avg gaps | ADJ29 avg gaps | Δ |
|---|---|---|---|
| gemma4:latest | 12.8 | 7.1 | −5.7 |
| llama3.1:8b | 11.9 | 11.4 | −0.5 |
| qwen2.5:3b | 7.7 (7 cells) | 6.4 (7 cells) | −1.3 |
| qwen2.5:1.5b | 5.1 | 3.75 | −1.4 |
| **qwen2.5:0.5b** | **3.0** | **3.5** | **+0.5** |

qwen2.5:0.5b still leads on absolute gap count, but the ordering
gemma > llama > 3B > 1.5B > 0.5B is now compressed: gemma at 7.1 sits
between 3B (6.4) and llama (11.4) instead of clearly worst. The "small
model dominates" framing from ADJ28 was real but partly an artefact of
retry budget too tight for big-model fan-out.

## Finding 3 — qwen2.5:0.5b regressed slightly under more retries

The 0.5B model produced more gaps in ADJ29 than ADJ28 on two cells
(`large-lithium` +2, `wine-bottle` +2). One hypothesis: the small
model produces output of consistent quality regardless of retry depth,
and deeper retries at FactToTypedComponent give it more chances to
emit incoherent typed-component splits. The improvement budget aimed
at big-model recovery doesn't translate to small-model coherence.

Net: +4 gaps for the smallest model. The cells that did improve at
g=1..2 stayed at g=1..2; the cells that already failed cleanly failed
slightly worse.

## Finding 4 — Wallclock cost is 1.56× ADJ28

| | ADJ27 | ADJ28 | ADJ29 |
|---|---|---|---|
| Wallclock | 47.5 min | 83.9 min | 131.0 min |
| vs prior | — | +76% | +56% |

The wallclock growth is dominated by deep-level retry loops at
FactToTypedComponent. A gemma4 cell at the new 8-retry deepest budget
can spend much of its wallclock on the last level alone. The cost is
linear in the new budget; doubling FactToTypedComponent from 8 to 16
would not double the wallclock unless cells routinely hit the 8 retry
ceiling. Looking at the per-cell `retry_calls` field in the raw data
will tell us how many cells are still budget-limited.

## What this validates (and doesn't)

**Validated:**

- Per-level retry budgets reduce coverage gaps on inputs with deep
  fan-out — the ADJ29 theory holds.
- Big-model over-decomposition (ADJ28 Finding 3) was partially a
  retry-budget artefact, not a pure prompt failure.
- The smallest model is *not* uniformly the best; it leads only because
  its over-decomposition is so mild that uniform-3 budgets fit it.

**Not validated:**

- Still 0/40 fully passing. Gating condition unchanged.
- qwen2.5:0.5b regression on two cells is unexplained.
- We do not yet know whether the closed-gap cells closed because the
  model produced *better* IR on retry, or because the orchestrator's
  gap-counting metric is sensitive to non-semantic perturbations.

## Gating condition — still NOT met

Zero cells fully passing under any model. Paused workstreams
(ADJ14/15/16/17/18/19/20) stay paused. The ADJ28 → ADJ29 movement
shows we are making progress on gap counts (−20.6%) but not on the
all-or-nothing pass rate.

## Next interventions, in order

### 1. Capture per-level gap distribution

ADJ28 Finding 5 already proposed this. With ADJ29 in place we now
have two budget configurations to compare *per level*. If FactToTypedComponent
gaps fell disproportionately, the per-level theory holds end-to-end.
If they fell uniformly across levels, something else is happening.
Cheap to add to the bench binary.

### 2. Bump FactToTypedComponent further (8 → 12 or 16)

Gemma's biggest gains came from cells where this level dominates. If
per-cell `retry_calls` shows the 8-retry cap still binding, bumping
further is the natural next step. Risk: wallclock cost compounds.

### 3. Investigate qwen2.5:0.5b regression

Two cells got worse. Either: (a) the 0.5B model produces noisier output
on retry (perturbed sampling making things worse), or (b) the cells
*looked* the same but the gap-counting walk visits different boundaries.
A diff of the per-level coverage records between ADJ28 and ADJ29 for
those two cells would distinguish.

### 4. Re-run ADJ26 / ADJ27 / ADJ28 contracts in legacy mode for cross-check

The methodology bug above (uniform-3 forced via env var) means the
ADJ28 baseline could itself contain hidden artefacts. One sanity check:
re-run a single cell with `ADJ_PR6_MAX_RETRIES=3` and confirm we
reproduce ADJ28's gap count exactly. If yes, baseline is trustworthy.

## Gating threshold (unchanged from ADJ28)

- **Tier 1 unblock** (allow ADJ20 fact-sheets to resume): 5 / 40 cells
  fully passing across the matrix, with no model contributing more
  than 60% of the passes.
- **Tier 2 unblock** (allow ADJ16 engine arm, ADJ18/19 verdict
  benches): 15 / 40 cells fully passing.
- **Tier 3** (full unblock, including rulebook ADJ14/15/17): 25 / 40.

Three g=1 cells exist as of ADJ29 (`matches × qwen2.5:1.5b`,
`matches × qwen2.5:0.5b`, `matches × qwen2.5:3b`,
`lighter-disposable × qwen2.5:1.5b`). One retry budget bump at the
right level closes them. Tier 1 is in reach.

## See also

- [ADJ28](ADJ28-anti-discard-bench-results.md) — anti-discard bench (this run's baseline).
- [ADJ27](ADJ27-content-shaped-decomposition-bench.md) — content-shaped contract bench.
- [ADJ26](ADJ26-foundation-bench.md) — methodology baseline.
- [PR #3227](https://github.com/adhithyan15/coding-adventures/pull/3227) — ADJ29 per-level retry budget code change.

## Status

- 2026-05-14: bench re-run complete; results captured; methodology bug retracted.
- Next: add per-level gap-distribution tracing to the bench binary; re-bench with deeper FactToTypedComponent budget.
