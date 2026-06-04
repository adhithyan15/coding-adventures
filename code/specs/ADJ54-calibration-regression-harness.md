# ADJ54 — Calibration-Regression Harness + First-Class Posterior Uncertainty

> **Status (2026-06-03):** Harness + per-case gate shipped and proven.
> Lever **H2** (open-question discounting) implemented in the runner and
> gated: **0 regressions**. Levers **H1** (correlation de-stacking) and
> **H3** (residual hypothesis) specified, not yet implemented. This spec
> is the milestone checkpoint after ADJ52's 100-case run and ADJ53's
> latent-mechanism construct.

## 1. The problem this fixes — fixes that add entropy

ADJ52's runs kept finding the same thing: the framework matches frontier
Claude on *correctness* and loses only on *calibration* — saturated
posteriors (median 0.99), ~99% confidence asserted *while recommending the
confirmatory test*, and confidence that does not track correctness.

The dangerous part is the **history of fixes for it**. The run-2 "softmax
coherent-differential" patch *made things worse* (framework 0/3) and
**nobody could see it**, because it was judged on the end-to-end blind-judge
loop at n=3. That loop fuses three different signals — correctness,
calibration, defensibility — into a single win/loss bit. A fix that helped
the aggregate while silently regressing individual cases slipped straight
through. The softmax patch tempered multi-hypothesis incoherence (good) but
worsened single-hypothesis saturation (bad), and when the top pick was a
wrong sibling it now *actively argued against the right answer*.

**That hidden per-case regression is the entropy we keep adding.** You do
not fix it by being more careful with the next patch. You fix it by changing
the instrument so a regression cannot hide.

## 2. The instrument — a deterministic, offline, per-case gate

The engine is **deterministic**: given a frozen `(rulebook.adj,
program.adj)`, the posterior is a pure function of the engine code. The
blind-judge loop throws that determinism away. So:

1. **Freeze a golden corpus** of `(rulebook, program, ground-truth label)`
   tuples — see §4. Built once, from a real ADJ52 diagnostic run.
2. **Score any engine change offline in milliseconds per case**, with
   metrics that *decompose* what the blind judge fused:
   - **Correctness** — does top-1 still match the ground-truth diagnosis?
   - **Calibration** — Brier, log-loss, ECE, saturation rate, and
     **confidently-wrong** (top-1 wrong at ≥ 0.90).
3. **The gate is per-case and signed, not aggregate.** A fix is rejected if
   *any* case flips correct→wrong, becomes newly confidently-wrong, or grows
   more confident while wrong — **even if the mean improves**. Aggregate-only
   gates are exactly how softmax got in.

Implementation: [`code/specs/data/adj52/calibration/score.py`](data/adj52/calibration/score.py).

```
python score.py score corpus.json out/before.json   # baseline
# ... change the engine, rebuild ...
python score.py score corpus.json out/after.json
python score.py diff out/before.json out/after.json  # THE GATE
```

The gate was validated both ways before use: it passes on identical runs,
and on a synthetically-regressed run it flags the regression **even when the
aggregate numbers are left unchanged** — i.e. it catches precisely the
softmax failure mode.

### Why aggregate metrics lie here

Scored on the 23 correct cases alone, Brier is **0.0006** — because
saturation and correctness reward each other (predicting ~1 when right is
"well-calibrated"). Scored on the full corpus including the wrong cases,
Brier is **0.16** and ECE **0.16**. The wrong cases *must* be in the corpus
or the aggregate flatters a broken model. This is why the gate weights the
per-case wrong set, not the mean.

## 3. The three levers (root-caused, not guessed)

A failure-enriched 30-case diagnostic re-run (ADJ52 pipeline, seeded toward
the run-3 failure specialties, **with per-case artifacts persisted** — which
run-3 discarded) produced 7 judged-wrong cases. Each was root-caused from its
*actual* rulebook + program + deterministic engine trace + blind-judge
rationale + ground truth. Two correct cases were included as controls. The
universal finding: **`open_uncertainty_present` AND `recommends_confirmatory_test`
are true in all 9 cases, including both correct controls** — the engine pins
~99% while recommending the very test that would confirm it. The wrong cases
split cleanly:

| Lever | Failure shape | Cases | Fix surface |
|---|---|---|---|
| **H1 — correlation de-stacking** | true answer is a *listed* sibling, suppressed to ~0 because 6–7 same-sign contributes over-stacked the wrong winner | case-5, case-7 | runner-level shrinkage on same-sign contribution stacks |
| **H2 — open-question discounting** | ~99% asserted while a decision-relevant `uncertain{}` bearing on the conclusion is unresolved | 6/9 cases (+ both controls) | runner-level VOI-band tempering |
| **H3 — residual hypothesis** | true answer is *not in the differential at all* (angiosarcoma / sporadic hemiplegic migraine / cardiac hibernoma) | case-6, case-18, case-29 | deriver-prompt: emit an explicit "other / none-of-the-above" hypothesis |

H1 is the saturation *mechanism*; H2 is the saturation *symptom*. The ADJ53
`mechanism` construct is the H1 lever where the deriver uses it — but the
deriver under-uses it, so saturation persists (availability ≠ use).

## 4. The golden corpus

[`code/specs/data/adj52/calibration/corpus.json`](data/adj52/calibration/corpus.json)
— 30 cases from the diagnostic re-run, each with its frozen `rulebook.adj` +
`program.adj` (in `cases/case-N/`) and a `correct_term` label. Labels for the
25 top-1-correct cases are the engine's own top-1 term; the 5 genuinely
misranked cases carry the true-diagnosis term from root-cause. Full per-case
run record (ground truth, perturbations, judge rationales) is preserved in
[`runs/run-4-diagnostic-30case-full.json`](data/adj52/runs/run-4-diagnostic-30case-full.json).

**Baseline (pre-fix):**

| metric | value |
|---|---|
| top-1 accuracy | 0.833 (25/30) |
| confidently-wrong | 5 |
| saturated ≥ 0.99 | 17/30 |
| ECE | 0.161 |
| Brier | 0.160 |
| log-loss | 0.683 |
| mean top posterior | 0.988 |

## 5. H2 — open-question discounting (shipped)

**Principle.** The engine must not report saturated certainty while a
decision-relevant confirmatory test bearing on *this* conclusion is still
unobserved — the test could still go either way. We build a **VOI band** over
the open uncertainty's outcomes (`{posterior} ∪ {sigmoid(logit + Δ)}` for each
candidate value of the bearing `uncertain{}` marker) and report the band's
**midpoint** ("the test could go either way") as the *calibrated* confidence.

**The anti-entropy guarantee — rank on raw, calibrate on reported.** The RAW
posterior (evidence as observed) is what the differential is *ranked* on; the
tempered band value is *reporting-only*. So H2 **cannot reorder the top-1**
— by construction it introduces zero correctness regression. The companion
scorer ranks on RAW and scores calibration on REPORTED. This is the formal
property the softmax patch lacked.

Implemented in [`code/specs/data/adj52/src/main.rs`](data/adj52/src/main.rs)
(runner-level, zero blast radius — same pattern as the ADJ53 `mechanism`
construct; the Miri-checked `logic-engine` crate is untouched).

**Gate result (baseline → H2):**

| metric | baseline | H2 | |
|---|---|---|---|
| top-1 accuracy | 0.833 | **0.833** | unchanged (monotonic) |
| **per-case regressions** | — | **0** | gate passes |
| saturated ≥ 0.99 | 17 | **12** | −5 |
| log-loss | 0.683 | **0.591** | −13% |
| ECE | 0.161 | 0.151 | − |
| confidently-wrong | 5 | **5** | unchanged |

**What the gate proved — two things at once:**

1. **H2 added zero entropy.** 0 regressions, accuracy unchanged. The
   structural opposite of softmax.
2. **H2 alone is insufficient, and the gate said *why* empirically.**
   Confidently-wrong stayed at 5 because the bands are *narrow*
   (case-5: [0.951, 0.999]) — the saturation is driven by over-stacked
   *observed* evidence (H1), not the pending test. H2 honestly tempers the
   broad population but cannot rescue the 5 hard cases. No theorising was
   needed to learn this; the instrument localised the remaining damage to H1
   (case-5, case-7) and H3 (case-6, case-18, case-29).

## 6. Next steps (not yet done)

- **H1 — runner-level same-sign-contribution shrinkage.** Diminishing returns
  on the k-th same-sign contribution, recomputed from the fired-clause deltas
  the runner already exposes. Frozen-corpus testable immediately. *Meant* to
  reorder (to un-suppress the true sibling), so the gate against the 25
  correct cases is the safety net. Sweep shrinkage strength; keep the
  strongest setting with **zero** regressions.
- **H3 — deriver residual hypothesis.** Teach the deriver to emit an explicit
  `diagnosis(other_…)` residual that absorbs mass from generic/ambiguous
  evidence. Requires re-derivation, so it is a separate workstream not
  measurable on the frozen corpus.
- **Promotion.** If H1/H2 prove out in the runner, consider promoting the
  semantics into `logic-engine` proper (with Miri) as a follow-up.

## 7. Invariants this establishes

1. **No fix ships without passing the per-case gate.** Aggregate improvement
   is necessary but not sufficient.
2. **Reporting is decoupled from ranking.** Calibration changes (H2) must not
   touch the differential order; only correctness-targeted levers (H1, H3)
   may reorder, and only under the gate.
3. **The corpus includes the failures.** A calibration corpus of only-correct
   cases is self-deceiving.
