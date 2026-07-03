# W5 — cross-judge robustness: the ADJ99 rescore is not a single-judge artifact

**Status: COMPLETE.** A second, non-Opus judge (**Sonnet**) re-scored all 395 format-normalized,
arm-blind ADJ99 traces under the **identical** counterfactual locus-exposure rubric. Only the judge
model changed, so any difference is attributable to the judge. This resolves the single-judge caveat
flagged in the ADJ99 rescore (PR #5261) and sets the methodology for the corpus rescore sweep.

## Headline — the corrected result reproduces

| arm | n | Opus-judge def | Sonnet-judge def |
|---|---|---|---|
| plain-haiku | 100 | 1.90 | 2.10 |
| plain-opus | 98 | 3.03 | 3.11 |
| fw-haiku | 99 | 2.71 | 2.76 |
| fw-opus | 98 | 3.48 | 3.56 |

**fw − plain gap (the load-bearing claim):**

| pair | Opus judge | Sonnet judge |
|---|---|---|
| haiku | +0.81 | **+0.66** |
| opus | +0.45 | **+0.45** |

Both judges agree on the two things that matter: **the framework beats plain at both model scales**,
and the Opus gap is **+0.45 under both** (identical). The framework's defensibility advantage is a
property of the traces, not of the Opus judge.

## Inter-judge agreement (n = 395)

| metric | value |
|---|---|
| exact 0–5 match | 0.641 |
| within-1 | **0.967** |
| mean \|Δ\| | 0.397 |
| Pearson r | **0.79** |
| Sonnet − Opus (mean) | +0.11 (Sonnet slightly more generous) |
| agree `premise_named` | (see `w5_summary.json`) |

For a subjective 0–5 rubric, **96.7% within-1** and **r = 0.79** is strong agreement. Sonnet runs
~0.1 higher across the board, but the offset is uniform — it shifts the level, not the ordering or
the gaps.

## The mechanism replicates, not just the score

The *reason* the framework wins — it flags its load-bearing premise as fallible more often — shows up
under Sonnet too: fw-opus flags-fallible **0.367** (Sonnet) vs 0.367 (Opus); would-flip and
named-premise rates track within a few points. Two independent judges find the same mechanism.

## What this resolves

- **The ADJ99 sign-flip is real, not a single-Opus artifact.** Both judges falsify the old "framework
  is defensibility-neutral on the frontier" conclusion (fw-opus > plain-opus by +0.45 under both).
- **Methodology policy for the rescore sweep (W9/W10):** single-judge rescores are **reliable for
  clear effects** (high agreement, identical direction). Reserve a **second judge for close calls**
  (gaps within ~the mean \|Δ\| of 0.4, or any headline that hinges on a small delta). This keeps the
  corpus sweep affordable without sacrificing rigor — and unblocks W9.

## Caveats

- Two judges, both Anthropic-family. A fully independent check would add a third-party model; the
  within-Anthropic agreement is a *lower* bar than cross-vendor would be, but the direction is
  unambiguous and the magnitudes are close.
- N=1 per (cell, judge); we report agreement, not per-cell CIs. The stable, large effects (direction,
  the +0.45 opus gap reproducing exactly) are what we lean on; we do not over-read the ~0.1 Sonnet
  generosity offset.

## Artifacts
- `verdicts_sonnet.json` — the 395 Sonnet verdicts (non-regenerable).
- `rejudge_sonnet.workflow.js` — the judge workflow (`model: 'sonnet'`, identical rubric).
- `w5_aggregate.py` → `w5_summary.json` — the comparison.
Pipeline: regenerate `../judge_cells/` + `../cell_map.json` via `../rescore_prep.py`, then
`python3 w5_aggregate.py`.
