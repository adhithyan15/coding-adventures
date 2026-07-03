# Paper 1 — work plan & status (plan-of-record)

> Companion to [`PAPER1-skeleton.md`](PAPER1-skeleton.md). The skeleton is the *argument*;
> this is the *remaining work*. Updated 2026-06-09 to reconcile the skeleton (written before
> the ADJ73–100 arc) with what has actually been built, and to absorb the ADJ99 defensibility
> rescore (PR #5261).

## Where the four experiments actually stand

The skeleton's `#47–#50` task pointers are **stale** (they resolve to unrelated merged feature
PRs, not these experiments). Real status, mapped to the ADJ corpus:

> **Update 2026-06-10:** all four experiments now have their load-bearing data. E2 ran (and was
> reframed — see below); the 200-item benchmark (run100 + run100b) landed for E3; E4's cross-judge
> completed. The only deferred item is E1's frontier ablation tier (H4), now in Limitations.

| # | experiment | real status | evidence on disk | what's left |
|---|---|---|---|---|
| E0 | two worked HLE runs (Palmyrene stable-error; hummingbird unstable-gap) | **DONE** | ADJ72 | — |
| E1 | mechanistic ablation — *justified-discards is the lever* | **PILOT + at-scale corroboration** | `adj73-omission-ablation/` (lever ≤3B); abstention via E3 benchmark; capability-floor via E2 | H4 frontier tier → **Limitations** (E1 §8); no blocking run |
| E2 | cost-to-correct, framework vs prose — *the headline* | **DONE** | `e2-correctability/` (localize null; fix/propagate + recurrence win; cost_to_correct.json) | — (manuscript prose) |
| E3 | cross-domain matrix — *same machinery, only the rulebook changes* | **DONE** | 200-item benchmark (run100 + run100b) + ADJ49/56/59/70/71 ballast | — (optional figure) |
| E4 | HLE/defensibility screening harness | **DONE** | ADJ99 rescore (PR #5261) + Sonnet cross-judge (`w5-crossjudge/`) | — |

## What the ADJ99 rescore (PR #5261) changed for the paper

E4's defensibility number was confounded: the original 0–5 rubric scored citation/traceability
density, and output **format** let a "blind" judge identify the arm with 100% accuracy. The
rescore (construct-valid locus-exposure rubric + format normalization) **reversed** the headline —
the framework's defensibility advantage *grew* (opus fw−plain gap −0.11 → **+0.45**), because the
framework ~doubles the rate of flagging the load-bearing premise as fallible at both model scales.

Consequences for the manuscript:
- **E4 must report the corrected metric**, not the original 0–5 traceability score.
- The format-confound finding is itself a **measurement-validity contribution** — it belongs in
  Threats-to-Validity (and as a standing discipline: deterministic leak-check + format
  normalization before trusting any arm-vs-arm judge delta; see `lessons.md`).
- It is still **single-judge** (n=1/cell). Not load-bearing until a second, ideally non-Opus,
  judge reproduces the direction → W5.

## Work items — all complete

| id | item | kind | status |
|---|---|---|---|
| **W1** | this doc + skeleton refresh | docs | ✅ done |
| **W2** | E2 correctability head-to-head spec | spec | ✅ done → **run done** (`e2-correctability/`) |
| **W3** | E3 cross-domain consolidation writeup | analysis | ✅ done (refreshed with the 200-item benchmark) |
| **W4** | E1 confirmatory ablation + abstention-gate spec | spec | ✅ done → closure in E1 §8 (H4 deferred) |
| **W5** | E4 cross-judge robustness run | experiment | ✅ done (`w5-crossjudge/`) |
| **W6** | cross-cutting protocol doc | docs | ✅ done |
| **B1–B3** | pre-register + build + run the 200-item benchmark | experiment | ✅ done (run100 + run100b) |

**Paper-1 data is complete.** What remains is manuscript prose (assemble the four experiments into
the skeleton's narrative) and optional polish (publication figures). The one scientific item placed
in Limitations rather than run: E1's H4 frontier omission tier.

## Cross-cutting requirements (tracked in W6, surfaced from the skeleton's Threats/Repro)

- **Contamination:** held-out / less-contaminated items; state it.
- **Cross-model arm:** make it deliberate per headline experiment (partly covered by
  fw-haiku/fw-opus + ADJ85 cross-family).
- **Error bars / n / scoring protocol:** ADJ95 flagged blind-judge noise on free-form answers.
- **One-command reproducibility:** every quantitative claim traces to a repo artifact
  (byte-provenance applied to the paper itself).
- **Measurement-validity guard:** the deterministic-leak-check + format-normalization discipline
  (from the ADJ99 rescore) as a standing rule for every arm-vs-arm judge comparison.

## Headline reminder

E2 (cost-to-correct, framework vs prose) is the most defensible single result and is now **complete**.
Its claim was sharpened by the run: the localize-by-reading panel is a **null** (a strong reviewer
reads prose as well as the trail, after the format-confound guard), so the headline rests on
**non-recurring cost-to-correct** — the fix lives in an editable artifact, paid once, propagating,
where stateless prose re-incurs the same error forever. The capability-graded recurrence result
(1.5B raw 0/7 → +framework 7/7) is the one-number expression of the program's thesis: *intelligence
accumulates in the framework, not the weights.*

## Out of scope here (paper 2)

MYCIN-2026 as the "derive once, reuse indefinitely + offload reasoning to CPU" proof is **paper 2**
(CPU-bound reasoning / ProbLog distillation). Planned separately in W7 → its own spec; not part of
paper 1's experiment set.
