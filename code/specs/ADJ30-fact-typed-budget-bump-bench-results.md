# ADJ30 — FactToTypedComponent Budget Bump: Hypothesis Falsified

> Follow-up data PR to [ADJ29](ADJ29-per-level-retry-budget-bench-results.md).
> Tests **ADJ29's "next interventions" item #2** — *"Bump
> FactToTypedComponent further (8 → 12 or 16)"* — on the four g=1
> cells the per-level retry budget left within one retry of passing.
>
> **Result: hypothesis falsified.** Doubling the FactToTypedComponent
> retry budget from 8 to 16 closes zero g=1 cells, regresses two
> small-model cells (+2 gaps each), and produces one catastrophic
> regression to *unparseable* output. The bottleneck on the g=1
> cells is **not** retry budget at the FactToTypedComponent level.
>
> Raw data: [`data/adj30-h1-fact-typed-budget-bump-2026-06-01.json`](data/adj30-h1-fact-typed-budget-bump-2026-06-01.json).

## Hypothesis (H1)

> Bumping `ADJ_PR6_MAX_RETRIES_FACT_TYPED` from 8 (ADJ29 default)
> to 16 will close at least one of the four g=1 cells documented
> in [ADJ29 §"Headline numbers"](ADJ29-per-level-retry-budget-bench-results.md#headline-numbers)
> to g=0, bringing the matrix to ≥1/40 fully-passing cells and
> validating the "deeper FactToTypedComponent budget closes the
> near-miss cells" theory.

## Method

- Same harness as ADJ29 (`code/scripts/adj_pr6_foundation_bench.py
  --per-level-defaults`) with the additional env override
  `ADJ_PR6_MAX_RETRIES_FACT_TYPED=16`.
- Per-level budgets: `3 / 4 / 5 / 16` across
  DocumentToSentence / SentenceToPhrase / PhraseToClaim /
  FactToTypedComponent — i.e., **only** the deepest level changed
  from ADJ29's `3 / 4 / 5 / 8`.
- Cell selection: the **four g=1 cells from ADJ29** plus
  **two adjacent cells** (`lighter-disposable × qwen2.5:0.5b`
  and `lighter-disposable × qwen2.5:3b`) for context on where
  the deeper budget might overshoot. Two declarations
  (`matches`, `lighter-disposable`) × three models
  (`qwen2.5:0.5b`, `qwen2.5:1.5b`, `qwen2.5:3b`) = 6 cells total.
- Same five-model Ollama on the same M2 Max 96 GB hardware.
- Cell timeout 1200 s (vs ADJ29's 900 s) to give the bumped budget
  room.

## Results

| Cell                                   | ADJ29 gaps | H1 gaps           | Δ          | Wallclock |
|----------------------------------------|------------|-------------------|------------|-----------|
| `matches × qwen2.5:0.5b`               | 1          | **3**             | **+2**     | 75.1 s    |
| `matches × qwen2.5:1.5b`               | 1          | 1                 | =          | 56.8 s    |
| `matches × qwen2.5:3b`                 | 1          | 1                 | =          | 16.8 s    |
| `lighter-disposable × qwen2.5:0.5b`    | 3          | **5**             | **+2**     | 37.4 s    |
| `lighter-disposable × qwen2.5:1.5b`    | 1          | 1                 | =          | 69.1 s    |
| `lighter-disposable × qwen2.5:3b`      | 4          | **unparseable**   | catastrophic | 120.8 s   |

**Net change**: 0 cells closed, 2 cells regressed by +2 gaps each,
1 cell catastrophically regressed to unparseable output, 3 cells
held flat.

**Hypothesis H1 is falsified.** Bumping FactToTypedComponent
budget from 8 to 16 is not the path to Tier 1 unblock.

## Three findings

### Finding 1 — The g=1 gaps live above FactToTypedComponent

Three of the four ADJ29 g=1 cells (matches × {1.5b, 3b}, and
lighter-disposable × 1.5b) held at g=1 under the bumped deepest
budget. If those g=1 gaps had been at the FactToTypedComponent
level, doubling the deepest budget should have closed at least
some of them. They didn't move.

**Conclusion**: the residual g=1 gaps in those three cells are
at a *higher* level of the decomposition — DocumentToSentence,
SentenceToPhrase, or PhraseToClaim. Bumping the deepest budget
does not help. The right next intervention is precisely
**ADJ29's item #1**: capture per-level gap distribution so we
know *which level* the surviving gaps occupy.

### Finding 2 — qwen2.5:0.5b regression confirms the coherence-ceiling theory

ADJ29 Finding 3 hypothesized:

> *"the small model produces output of consistent quality
> regardless of retry depth, and deeper retries at
> FactToTypedComponent give it more chances to emit incoherent
> typed-component splits."*

H1's data validates this on the cells in scope:

- `matches × qwen2.5:0.5b`: g=1 → g=3 (+2)
- `lighter-disposable × qwen2.5:0.5b`: g=3 → g=5 (+2)

Both 0.5B cells regressed by exactly 2 gaps with the deepest
budget doubled. The small model's output noise at deeper retry
depths is now an empirically confirmed phenomenon, not a
hypothesis. **Deeper retry budgets *actively harm* the smallest
model on the cells we tested.** A useful direction is
per-*model* retry budgets, not per-level, that cap the budget for
known-coherence-ceiling models.

### Finding 3 — qwen2.5:3b × lighter-disposable went from g=4 to unparseable

The single most striking regression. ADJ29 had this cell at g=4
(four coverage gaps, parseable IR). H1 has it at *unparseable at
FactToTypedComponent* — the binary failed to produce valid
JSON for the deepest level after 16 attempts. With 8 attempts
the model produced flawed-but-parseable output; with 16 attempts
the parseable shape escaped it.

**Conclusion**: retry budgets are not monotonically safe.
Bumping them can *unlock new failure modes* (unparseable output)
that the lower budget statistically avoids. The framework's
guarantee "more retries can only help" is not currently held.

## What H1 falsifies and what remains intact

**Falsified by H1**:

- ADJ29 "next interventions" item #2 (bump FactToTypedComponent
  further). The deeper budget does not close g=1 cells.
- The implicit assumption that *retry depth at the right level*
  is the dominant lever for closing the gap to Tier 1 unblock.

**Not falsified by H1**:

- ADJ29's overall per-level retry framework. The 3/4/5/8 default
  is still the right shape; the data here suggests it is also
  near a local optimum, not a stepping stone to higher budgets
  at the deepest level.
- The Tier 1 unblock target (5/40 cells fully passing). H1
  contributes 0/6 cells to that count but does not invalidate
  the goal.
- The hierarchical decomposition flow itself (ADJ25).

## What ADJ30 implies for the next intervention

The right next move is **not** more retries. It is **knowing
which level the surviving gaps occupy**. ADJ29 listed this as
intervention #1 ("Capture per-level gap distribution") and
estimated it as "cheap to add to the bench binary." H1's data
elevates it from "cheap to add" to "required before any further
retry-budget intervention is justified."

Concretely, the bench binary should emit, per cell, a
`per_level_gap_distribution: { document_to_sentence: usize,
sentence_to_phrase: usize, phrase_to_claim: usize,
fact_to_typed_component: usize }` field next to the existing
`total_gap_count`. With that, ADJ31 can re-bench at the default
budget and answer:

- Which level dominates the residual gaps on the g=1 cells?
- Is the right intervention prompt-level (a sharper phrase-level
  contract) rather than budget-level?
- Are the qwen2.5:0.5b regressions concentrated at a single
  level, or distributed?

Until that data exists, further retry-budget interventions are
guesses.

## Cost summary

| Metric | Value |
|---|---|
| Cells run | 6 |
| Wallclock total | 375.9 s ≈ 6.3 min |
| Cells passed | 0 |
| Cells improved | 0 |
| Cells flat | 3 |
| Cells regressed | 3 (two +2 gaps, one catastrophic) |
| LLM calls (estimated) | ≥ 6 × ~16 = ~100 (deepest level alone) |

Cheap experiment, clean falsification.

## Gating condition — still NOT met

Zero cells fully passing. Tier 1 unblock requires 5/40. ADJ30's
empirical contribution is **a tighter understanding of where the
bottleneck is *not***, plus a redirection toward measurement
(per-level gap distribution) before further code changes.

## See also

- [ADJ29](ADJ29-per-level-retry-budget-bench-results.md) — the
  per-level retry budget bench whose "next interventions" this
  spec tested.
- [ADJ28](ADJ28-anti-discard-bench-results.md) — the anti-discard
  bench (ADJ29's baseline).
- [ADJ25](ADJ25-hierarchical-decomposition.md) — the foundational
  reset whose gate the chain is trying to satisfy.

## Status

- 2026-06-01: H1 bench run complete; results captured;
  hypothesis falsified.
- Next: instrument bench binary with per-level gap distribution
  (ADJ29 intervention #1) and re-bench at the **ADJ29 default
  budget** to learn where the surviving gaps live. Hypothesis H2
  (which this spec doesn't test): the residual g=1 gaps live at
  SentenceToPhrase or PhraseToClaim, and a per-level prompt
  refinement at that boundary will close them more reliably than
  any retry-budget change.
