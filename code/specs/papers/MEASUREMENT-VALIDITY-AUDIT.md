# Measurement-validity audit & rescore sweep (plan-of-record)

> The ADJ99 rescore (PR #5261) showed its defensibility metric was **format-confounded** and measured
> the **wrong construct** — and fixing it *reversed* the headline. Those are **patterns, not a
> one-off**: any older run that judged format-differing arms with one blind LLM judge, or scored
> accuracy with an LLM judge, is suspect by the same logic. This doc turns "probably ran into the
> same problems" into a **ranked, evidence-grounded worklist**, and defines the rescore discipline.
> Companion to [`PAPER1-methods-protocol.md`](PAPER1-methods-protocol.md) and `lessons.md`.

## 1. The systemic failure modes (the screen)

| # | failure mode | tell | first caught |
|---|---|---|---|
| F1 | **Format confound** — arms differ in output format; a "blind" judge reads the arm off the style | a regex predicts the arm from raw output > chance | ADJ99 (100% regex separation) |
| F2 | **Wrong construct** — "defensibility" scored as citation/traceability density, not locus-exposure | def≥4 mostly *wrong*; score ⊥ correctness only by accident | ADJ99 (70% of def≥4 wrong) |
| F3 | **Single judge** — one blind LLM judge; no inter-judge check | n=1/cell; no agreement number | ADJ95 / ADJ99 caveat |
| F4 | **Grader noise / brittle matcher** — LLM accuracy judge, or token matcher that breaks on style | "effect" vanishes on deterministic re-grade | ADJ95 (fake 1→3), ADJ73 (token matcher) |

**The screen (apply to every judged run):** deterministic leak check → format normalization →
construct-valid rubric → ≥2 judges → deterministic/style-invariant correctness re-grade. (Full rules:
[`PAPER1-methods-protocol.md`](PAPER1-methods-protocol.md).)

## 2. Triage — at-risk ADJ runs

Risk = which failure modes apply. **Raw?** = per-item raw outputs preserved (cheap re-score) vs needs
re-run. Tier = rescore priority.

| run | metric / design | risk | raw? | rescore path | tier |
|---|---|---|---|---|---|
| **adj86**-defensibility-benchmark | 2×2 {Haiku,Opus}×{bare,framework}; blind judge **+ pairwise "more_defensible"** | F1,F2,F3 — *pairwise is extra format-leaky* (judge sees both; only fw is structured). **Origin of the rubric.** | ✅ | re-judge bare/fw with normalization + locus-exposure rubric; redo pairwise blind to format | **1** |
| **adj95**-defensibility-pilot | fw vs plain, blind Opus 0–5, N=1; already flagged accuracy grader-noise | F1,F2,F3,F4 | ✅ | rescore defensibility (construct+normalize); deterministic accuracy already partly done | **1** |
| **adj84**-pipeline-defensibility | staged pipeline defensibility | F1,F2,F3 | ✅ | rescore under locus-exposure + normalization | **1** |
| **adj87**-hle-benchmark / **adj88**-hle-run10 | HLE defensibility pilots (adj88 has a contaminated v1) | F1,F2,F3 | ✅ | rescore; adj99 already supersedes the 100-item version | **1** |
| **adj72** | E0 worked HLE runs (defensibility) | F2,F3 | ✅ | rescore defensibility; these are *cited* worked examples — must be construct-valid | **1** |
| adj89-opus-bp-coverage, adj90-support-convergence, adj92-closedbook-lift, adj93-opus-spider-haiku-reason | accuracy / coverage with LLM judge | F4 (±F3) | ✅ | **deterministic re-grade** from raw + gold | **2** |
| adj96-auditability, adj97-self-audit, adj98-adversarial-haiku | error-*localization* vs an **oracle** (not arm-vs-arm defensibility) | lower (F1 partial) | ✅ | mostly sound; add a format-normalization check on the trail-vs-prose localize comparison | **3** |
| adj52, adj57 | pipeline / CAS demos (not primarily judge-scored) | low | ✅ | spot-check only | **3** |

**adj99-hle100-run: DONE** (PR #5261) — the template for the sweep.

## 3. Methodology gate — W5 decides single vs multi judge for the whole sweep

The sweep's cost hinges on one open question: **is a single-judge rescore trustworthy?** Work item
**W5** (running now) re-judges the ADJ99 traces with a second, non-Opus judge (Sonnet).
- If Sonnet **agrees** with Opus (high exact/within-1, sign-flip reproduces) → single-judge rescores
  are reliable; the Tier-1 sweep can run one judge per run (cheap).
- If Sonnet **diverges** → the metric is judge-sensitive; **every rescore in the sweep needs ≥2
  judges** (and the ADJ99 result itself needs the multi-judge number as primary). More expensive, but
  non-negotiable.

**Do not launch the Tier-1 sweep until W5 returns.** (Tier-2 deterministic re-grades have no judge,
so they can proceed independently.)

## 4. Why this is a contribution, not just cleanup

Auditing our own benchmark corpus, finding the confounds, and re-scoring is the paper's ethos applied
**reflexively**: *auditable and correctable, not assumed-correct.* "We discovered our defensibility
metric was format-confounded, audited the whole corpus against the failure mode, and report the
corrected numbers with inter-judge agreement" is a **rigor/threats-to-validity asset**, not an
embarrassment. It is the strongest possible demonstration that the framework's discipline works —
including on its authors.

## 5. Work items (tracked)
- **W8** — this doc (the triage + discipline).
- **W9** — Tier-1 defensibility rescores (gated on W5).
- **W10** — Tier-2 deterministic accuracy re-grades (independent of W5).
- **W11** — domain-expansion plan (more cross-domain runs, with the screen baked in from day one —
  *don't add domains under a broken metric*).

## 6. Sequencing
1. W5 returns → set single-vs-multi-judge policy.
2. W10 (deterministic re-grades) can run in parallel — no judge needed.
3. W9 Tier-1 in priority order: **adj86 first** (it defined the rubric; fixing the origin is highest
   leverage), then adj95/84/87/88/72.
4. Each rescore mirrors ADJ99: preserve raw, normalize format, construct-valid rubric, report
   old-vs-new headline + inter-judge agreement, banner the original FINDINGS.
5. Only then W11 domain expansion, with the corrected metric and multi-judge default.
