# Paper 1 · methods protocol (reproducibility, scoring, contamination, measurement-validity)

> **Work item W6** (docs). The cross-cutting methods discipline every paper-1 experiment must follow,
> distilled from hard-won lessons (ADJ73, ADJ95, ADJ99 rescore, ADJ54). Surfaces the skeleton's
> *Threats to validity* and *Reproducibility* sections into enforceable rules.
> Plan: [`PAPER1-WORKPLAN.md`](PAPER1-WORKPLAN.md).

## 1. Reproducibility — every quantitative claim traces to a repo artifact

Byte-provenance applied to the paper itself. Rules:
- **One-command harness per experiment.** Each E#/W# run ships a runnable harness under
  `code/specs/data/<run>/` (script + raw outputs + `FINDINGS.md`), the layout the ADJ runs already
  use. A reader re-runs with one command.
- **Save all raw model outputs**, never just the scored aggregate. ADJ73's and ADJ99's correctness
  corrections were only possible because raw generations were preserved for deterministic re-scoring.
- **Every number in the manuscript carries a pointer** to its source artifact (`code/specs/data/adj*/…`).
  No figure number that can't be traced to a file.
- **Determinism by default:** temp 0; record model id + prompt version/hash (the ADJ audit-trail
  pattern) so a claim re-derives at disk speed.

## 2. Correctness scoring — never trust a single LLM accuracy judge

The recurring failure (ADJ92/94/95): an LLM accuracy judge on free-form / numeric / short answers is
**noise-dominated**, and that noise manufactured fake effects (ADJ95: "Opus-CAS triples Haiku's
accuracy 1→3" was a **grader artifact** that vanished on deterministic re-scoring; ADJ73: a brittle
token matcher scored "$0.00"/"No sales tax applies" as wrong and biased the experiment *against* the
framework). Rules:
- **Score deterministically from saved raw outputs** wherever the answer is checkable (numeric,
  short, exact). Preserve raw → re-grade offline.
- **Style-invariant matching** when conditions change output style (the justified/CAS arms emit NL
  answers). A token match that depends on format is forbidden.
- If an LLM judge is unavoidable, treat its accuracy verdicts as **approximate**, report the
  deterministic re-score as primary, and **gold + raw answers must be preserved** for re-grading.
- Drop items whose **gold is itself wrong** (ADJ96 `integral` 5482) — log, don't score.

## 3. Defensibility scoring — construct-valid + format-normalized (the ADJ99 lesson)

Defensibility is **not** citation/traceability density. Measure **locus-exposure**: is the
load-bearing premise surfaced and flagged as fallible (so it can be overridden and re-derived)?
Mandatory guards for *any* arm-vs-arm judged comparison (E2 and E4 especially):
1. **Deterministic leak check first.** A regex/string classifier that tries to predict the arm from
   the raw artifact. If it beats chance, the judge is not blind. (ADJ99: a one-line regex on
   `{RETRIEVED FACTS, REASONING CHAIN, [cites, (src:}` separated the arms with **100%** accuracy.)
2. **Format normalization.** Render all arms into one envelope, strip distinguishing chrome, so the
   judge scores substance not style. (Skipping this inverted ADJ99's headline.)
3. **Construct-valid rubric.** Score named-pivot / flagged-fallible / would-flip-if; explicitly do
   **not** reward citations, length, fluency, or correctness. Keep the metric correctness-decoupled.
4. **Multi-judge.** A judged delta is not load-bearing until a second, ideally non-Opus, judge
   reproduces the direction (W5). Report inter-judge agreement.

See [`lessons.md`](../../../lessons.md) — "ADJ benchmarks — output format leaks the arm."

## 4. Contamination

Public benchmarks may be in training data. Rules:
- Prefer **held-out / less-contaminated** items; where public items are used, **state it** and treat
  closed-book recall as out-of-scope (the framework is open-book by construction — see
  `feedback_framework_openbook_reasoning_not_recall`).
- Never score the framework **closed-book against final-answer correctness** — that is the axis it is
  built to lose; it targets auditable/correctable reasoning, not recall.

## 5. Error bars, n, and cross-model

- **Bootstrap 95% CIs** on every reported rate; state n per cell. Pilots (small n) are labeled as
  pilots and not over-read (ADJ99 caveat: don't over-read a 3.61 vs 3.72 delta).
- **Pre-register** the planned comparisons per experiment (E1 H1–H4, E2 RQ1–3, E4).
- **Cross-model arm is mandatory**, not optional — addresses the skeleton's single-family threat.
  Every headline effect is shown at ≥2 model scales / families (the ADJ99 rescore mechanism held at
  both Haiku and Opus; ADJ85 replicated cross-family).

## 6. Calibration regression

Probabilistic verdicts (ProbLog/ADJ65 paths) are checked against the **calibration-regression
harness** ([ADJ54](../ADJ54-calibration-regression-harness.md)) so a refactor can't silently degrade
posterior calibration. Any experiment emitting probabilities cites its calibration check.

## 7. Ethics / data
Public data only; no real PHI (clinical items are synthetic or published de-identified cases); no IRB
needed. State this in the manuscript.

## 8. Checklist (apply to every E#/W# run before it is paper-grade)
- [ ] one-command harness + raw outputs saved under `code/specs/data/<run>/`
- [ ] deterministic / style-invariant correctness scoring; gold + raw preserved
- [ ] (if judged arms) leak check + format normalization + construct-valid rubric + 2nd judge
- [ ] contamination stated; not scored closed-book on recall
- [ ] bootstrap CIs + n; comparisons pre-registered; ≥2 models
- [ ] every quoted number points to its artifact
