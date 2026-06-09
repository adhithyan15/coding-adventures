# Paper 1 — work plan & status (plan-of-record)

> Companion to [`PAPER1-skeleton.md`](PAPER1-skeleton.md). The skeleton is the *argument*;
> this is the *remaining work*. Updated 2026-06-09 to reconcile the skeleton (written before
> the ADJ73–100 arc) with what has actually been built, and to absorb the ADJ99 defensibility
> rescore (PR #5261).

## Where the four experiments actually stand

The skeleton's `#47–#50` task pointers are **stale** (they resolve to unrelated merged feature
PRs, not these experiments). Real status, mapped to the ADJ corpus:

| # | experiment | real status | evidence on disk | what's left |
|---|---|---|---|---|
| E0 | two worked HLE runs (Palmyrene stable-error; hummingbird unstable-gap) | **DONE** | ADJ72 | — |
| E1 | mechanistic ablation — *justified-discards is the lever* | **PILOT done, mixed** | `adj73-omission-ablation/` | confirmatory run + **abstention gate** (W4) |
| E2 | correction-loop — *localize → fix → persist; framework vs prose* | **parts only** | ADJ96/97/98, ADJ99 audit-trail, ADJ-CAS edit-override (`8ca017932`) | assemble the **head-to-head cost-to-correct study** (W2) — *the headline* |
| E3 | cross-domain matrix — *same machinery, only the rulebook changes* | **data scattered** | ADJ49/56/59/70/71 + clinical demo | single **consolidated results artifact** (W3) |
| E4 | HLE/defensibility screening harness — *blind controls; correctness + defensibility + error bars; cross-model byte-stability* | **data in; metric just corrected** | ADJ87–100; rescore PR #5261 | **cross-judge robustness** pass + adopt corrected rubric (W5) |

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

## Remaining work items (one PR each)

| id | item | kind | depends on | priority |
|---|---|---|---|---|
| **W1** | this doc + skeleton refresh | docs | — | done-in-this-PR |
| **W2** | E2 correctability head-to-head **spec** | spec | — | **P0 (headline)** |
| **W3** | E3 cross-domain **consolidation** writeup | analysis (existing data) | — | P1 |
| **W4** | E1 confirmatory ablation + abstention-gate **spec** | spec | — | P1 |
| **W5** | E4 **cross-judge robustness** run (non-Opus 2nd judge over normalized ADJ99 traces) | experiment (Workflow) | #5261 | P1 |
| **W6** | cross-cutting **protocol** doc (reproducibility / correctness-scoring / contamination) | docs | — | P2 |

Specs (W2/W4) precede their compute runs deliberately — *Specs → Tests → Implementation*
(repo standard). W3 and W5 are the two that touch real result data; W3 only aggregates what
exists, W5 is the one new LLM run.

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

E2 (cost-to-correct, framework vs prose) remains the most defensible single result — it measures
what nobody else measures and can't be scooped by accuracy or hallucination-detection work. It is
the **rate-limiter on the paper** and is currently only assembled in parts. Build it next (W2 spec,
then the run).

## Out of scope here (paper 2)

MYCIN-2026 as the "derive once, reuse indefinitely + offload reasoning to CPU" proof is **paper 2**
(CPU-bound reasoning / ProbLog distillation). Planned separately in W7 → its own spec; not part of
paper 1's experiment set.
