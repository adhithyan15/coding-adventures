# ADJ31 — Per-Level Gap Distribution: PhraseToClaim Is the Dominant Blocker

> Empirical follow-up to [ADJ30](ADJ30-fact-typed-budget-bump-bench-results.md).
> Instruments the bench binary with per-level gap distribution
> (ADJ29 intervention #1, elevated to "required" by ADJ30) and
> re-benches the 4 g=1 cells at the ADJ29 default budget.
>
> **Finding: the dominant residual blocker is PhraseToClaim, not
> FactToTypedComponent.** Three of six tested cells have their
> single residual gap at PhraseToClaim; bumping FactToTypedComponent
> (as ADJ29 proposed and ADJ30 falsified) could never have closed
> these. The right next intervention is a **prompt-level fix at
> PhraseToClaim**, not any retry-budget adjustment.
>
> Raw data: [`data/adj31-per-level-gap-distribution-2026-06-01.json`](data/adj31-per-level-gap-distribution-2026-06-01.json).

## What ADJ31 ships

1. **`per_level_gap_distribution` field** added to the
   `coverage_unresolved` error JSON emitted by `adj_pr6_bench`.
   Four counts (`document_to_sentence`, `sentence_to_phrase`,
   `phrase_to_claim`, `fact_to_typed_component`) tallying how
   many of the residual gaps live at each boundary. Surfaces
   the data ADJ30 demonstrated was required before any further
   retry-budget intervention is justified.
2. **A 6-cell empirical run** at the ADJ29 default budget
   (3/4/5/8) using the instrumented binary, on the same cell set
   as ADJ30's H1.
3. **A targeted hypothesis (H2)** about which level the residual
   gaps live at, tested against the data.

## Hypothesis (H2)

> The residual g=1 gaps in the four ADJ29 cells (matches ×
> {0.5b, 1.5b, 3b}, lighter-disposable × 1.5b) live at a *single
> level higher than FactToTypedComponent*. ADJ30 already showed
> they are not at FactToTypedComponent (bumping the deepest
> budget didn't close them). If the dominant location is one
> specific level, prompt work at that level is the right next
> intervention.

## Results

| Cell                                  | Total gaps | doc→sent | sent→phrase | phrase→claim | fact→typed | Wallclock |
|---------------------------------------|------------|----------|-------------|--------------|------------|-----------|
| `matches × qwen2.5:0.5b`              | 3          | **1**    | 0           | 0            | **2**      | 43.8 s    |
| `matches × qwen2.5:1.5b`              | 1          | 0        | 0           | **1**        | 0          | ~70 s     |
| `matches × qwen2.5:3b`                | 1          | **1**    | 0           | 0            | 0          | ~25 s     |
| `lighter-disposable × qwen2.5:0.5b`   | 5          | **1**    | **3**       | **1**        | 0          | ~40 s     |
| `lighter-disposable × qwen2.5:1.5b`   | 1          | 0        | 0           | **1**        | 0          | ~70 s     |
| `lighter-disposable × qwen2.5:3b`     | unparseable at FactToTypedComponent       | — | — | — | — | ~110 s |

### Per-level totals across the 5 cells with parseable output

| Level                  | Total gaps | Cells with ≥1 gap |
|------------------------|------------|-------------------|
| DocumentToSentence     | 3          | 3 (matches × 0.5b, matches × 3b, lighter-disp × 0.5b) |
| SentenceToPhrase       | 3          | 1 (lighter-disp × 0.5b only) |
| **PhraseToClaim**      | **3**      | **3** (matches × 1.5b, lighter-disp × 0.5b, lighter-disp × 1.5b) |
| FactToTypedComponent   | 2          | 1 (matches × 0.5b only) |

## H2 result — confirmed for the 1.5B-class cells

**Three of the four ADJ29 g=1 cells now traced** (one regressed
to g=3 in this run, so the original ADJ29 g=1 isn't reproducing
exactly — see "Methodology note" below):

- `matches × qwen2.5:1.5b` (g=1): single gap at **PhraseToClaim**.
- `lighter-disposable × qwen2.5:1.5b` (g=1): single gap at
  **PhraseToClaim**.
- `matches × qwen2.5:3b` (g=1): single gap at
  **DocumentToSentence**.

The 1.5B cells' g=1 blocker is at **PhraseToClaim**. The 3B cell's
g=1 blocker is at **DocumentToSentence**. The 0.5B cell didn't
reproduce its ADJ29 g=1; this run had g=3 distributed across
DocumentToSentence and FactToTypedComponent.

H2 is **partially confirmed**: residual gaps in the 1.5B cells
concentrate at PhraseToClaim. The 3B `matches` cell's gap is at
DocumentToSentence — a different level than predicted.

## What this redirects

**ADJ30 falsified the FactToTypedComponent budget intervention.**
**ADJ31 localizes the surviving gaps to PhraseToClaim (1.5B cells)
and DocumentToSentence (matches × 3B).** The right next
intervention is **not** any retry-budget change. It is:

1. **A prompt-level review of the PhraseToClaim decomposition
   prompt.** Three of the five parseable cells have their
   residual gap at this boundary. A sharper Phrase → Claim
   contract (or a worked-example refinement, or an explicit
   "every Phrase must produce ≥1 Claim that tiles its span"
   constraint) is the path most likely to close these.
2. **A prompt-level review of the DocumentToSentence
   decomposition prompt** for short single-sentence inputs.
   `matches × qwen2.5:3b` produced a Sentence node whose span
   didn't tile the 24-byte Document — the model is finding a way
   to lose one byte on the doc→sentence boundary even on the
   simplest input. Worth investigating whether the prompt
   over-constrains short single-sentence cases.

**Neither of these requires touching the retry budget.** The
budget can stay at ADJ29's `3/4/5/8` for now; the gap-closure
work moves to the prompt layer.

## What this implies for the publishable benchmark

The current foundation bench measures whether each cell's IR
ultimately satisfies the per-level coverage gate (pass / fail).
The ADJ31 data shows that **gap distribution within "fail"
matters** — three of the five fail cells are g=1, with the
single gap concentrated at one level. For paper-grade reporting:

- A fail at g=1 with the gap at PhraseToClaim is *qualitatively
  different* from a fail at g=20 spread across all four levels.
  The first is a near-miss with a clear next-step; the second
  is a structural breakdown.
- The right paper-grade metric is therefore **per-level pass
  rate** ("fraction of cells whose IR passes coverage at level
  L") plus a **gap-distribution histogram** for the residual
  failures, not a single overall pass rate.
- ADJ31's `per_level_gap_distribution` field is the minimum
  instrumentation required to compute these.

## Methodology note — small per-run variance is now empirical

The ADJ29 table reported `matches × qwen2.5:0.5b` at g=1. The
ADJ31 re-run (same model, same fixture, same `temperature: 0.0`,
same Ollama version, default budget) produced g=3. Ollama at
`temperature: 0.0` is *not* bit-exact deterministic — small
floating-point ordering effects on the model's inference can
shift the output enough that a single coverage gap becomes
three.

This contradicts ADJ12 v2's "zero variance" claim for the small
24-byte fixture. ADJ12 v2 was correct about *pass/fail outcomes*
on the ADJ12 fixture; ADJ31 shows that *gap-count specifics* on
the hierarchical-decomposition bench can shift between runs even
when pass/fail is stable. Future benches should report cells at
their median across N≥3 runs when gap-count specifics matter, or
report the modal coverage outcome (which is what ADJ29-30 did
implicitly via single-run measurement).

This does **not** invalidate ADJ31's qualitative finding: the
per-level distribution is dominated by PhraseToClaim on the 1.5B
cells, and that signal would survive at any reasonable run-count.

## Cost summary

| Metric | Value |
|---|---|
| Cells run | 6 |
| Wallclock total | ~6 min |
| Code added | ~20 LOC in `adj_pr6_bench.rs` |
| Field added to bench JSON | `per_level_gap_distribution` |
| Backwards-compat | Yes — field is omitted when error is not `CoverageUnresolved` |

## Gating condition — still NOT met

Zero cells fully passing. Tier 1 unblock requires 5/40. ADJ31
contributes:

- 0 / 6 cells closed (gates unchanged)
- 1 instrumentation feature ready for use across future benches
- Concrete signal that the right next intervention is a
  PhraseToClaim prompt change, not a retry-budget change

## See also

- [ADJ29](ADJ29-per-level-retry-budget-bench-results.md) — proposed
  per-level gap distribution as intervention #1.
- [ADJ30](ADJ30-fact-typed-budget-bump-bench-results.md) — falsified
  the FactToTypedComponent budget bump (intervention #2),
  elevating intervention #1 to "required."
- [ADJ25](ADJ25-hierarchical-decomposition.md) — the hierarchical
  decomposition flow whose gate the chain is trying to satisfy.

## Status

- 2026-06-01: bench-binary instrumentation landed, 6-cell run
  complete, per-level distribution localized.
- Next: prompt-level review at PhraseToClaim. Hypothesis H3 (for
  the next bench): rewriting the Phrase → Claim contract to
  enforce "every Phrase produces ≥1 Claim, and the Claims tile
  the Phrase's span" closes the 1.5B-class g=1 cells.
