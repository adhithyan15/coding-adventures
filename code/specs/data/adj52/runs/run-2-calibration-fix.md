# ADJ52 — run 2 (with the coherent-differential calibration fix)

Same 3 perturbed cases, hands-off (Workflow `wol4v13xw`), now with the
softmax-normalized coherent differential + deriver-tagged exclusivity + the
rulebook/program separation + disposition in the answer.

## Scorecard

| | run 1 | run 2 (this) |
|---|---|---|
| framework won | 1/3 | **0/3** |
| plain Claude won | 2/3 | **3/3** |
| framework correct | 2/3 (1 partial) | 2/3 (1 partial) |
| plain correct | 2/3 | 2/3 |
| compile failures | 0 | 0 |
| perturbation preserved dx | 3/3 | 3/3 |

The fix did **not** flip the losses — it went the other way (1→0). Small N, and
the perturbations differ run-to-run, but the *reason* is consistent and clear.

## What the blind judge said (all 3 cases)

- **case-1 (McArdle):** both correct. Plain won — it named the exact PYGM
  mutation and gave honest ~75%/~65% confidence; the framework still showed an
  **over-saturated ~100%** top *plus a spurious 99.2% "coexisting PMR"* and
  pseudo-precise logits ("+2.20, −1.90, +18.1") the judge called "fabricated
  quantitative scaffolding."
- **case-2 (Zenker diverticulum):** framework partial **but wrong sibling** —
  it committed **~100% to Killian-Jamieson and put the correct answer (Zenker)
  at ~0%, actively arguing against it.** Plain won by naming Zenker in its lead
  with honest 55–65% confidence and recommending CT (the true confirmatory test).
  Last run the framework's KJD call *won* (plain erred); this run the same
  overconfidence **confidently excluded the right answer** and lost.
- **case-3 (leprosy):** both correct. Plain won — honest ~90% vs the framework's
  **100%/0% split before the confirmatory smear it itself recommends** (the judge:
  "internally inconsistent with its own recommended next step"), plus fabricated-
  looking PMIDs and "66.7%/97%/98%" precision.

## The real finding (this run is the proof)

1. **The softmax fix tempered multi-hypothesis incoherence but NOT
   single-hypothesis saturation.** When the deriver's LRs make one hypothesis
   dominate, the normalized top still hits ~100% and now the differential
   **confidently excludes competitors at ~0%** — which is *worse* when the top
   pick is a wrong sibling (case-2: 100% KJD vs 0% Zenker). The problem moved; it
   wasn't solved.
2. **"100% while recommending the confirmatory test" is the core incoherence.**
   The framework asserts certainty *and* says "get the smear/genetics to
   confirm." A calibrated reasoner holds residual probability until that test
   returns. This is exactly the "uncertainty at the core" gap: the engine has
   the `uncertain`/VOI machinery but the posterior still collapses to 100%
   because the observed evidence dominates and nothing discounts for the
   *unresolved confirmatory* uncertainty.
3. **Pseudo-precise numerics + unverifiable citations are read as a NEGATIVE**
   ("false rigor," "look hallucinated") — the audit trail's apparent precision
   hurts more than it helps when the numbers aren't grounded/verifiable.
4. **The "plain" arm is frontier Claude reasoning fully and is well-calibrated.**
   It names exact mutations, includes the right answer in its lead, gives graded
   confidence, infers "stop the methotrexate." Out-diagnosing it on a blind
   comparison is a very high bar — and diagnostic correctness alone (which both
   arms share) doesn't win.

## Implication (strategic, honest)

Against a strong, well-calibrated base model, a **blind "who's the better
diagnostician" comparison is the wrong success metric** — the framework reaches
the correct answer with a full audit trail and still loses 0/3, on calibration
and false-precision. Two paths forward:

- **Deep calibration fix (engine):** the posterior must NOT reach ~100% while an
  open confirmatory `uncertain` marker is outstanding — discount the posterior by
  the unresolved-confirmation uncertainty, and stop displaying raw logits as if
  measured. This is a real engine change (temper LR magnitudes and/or hold
  residual mass for unobserved decision-relevant tests), and it directly serves
  "uncertainty at the core."
- **Reposition the metric:** the framework's value was demonstrated (ADJ17) when
  the *answerer is a small/weak model* reusing the rulebook — not against frontier
  Claude. Test the framework wrapping a small answerer vs that same small model
  raw; and measure **auditability + error-catching + thin-rulebook robustness**
  (ADJ50 regime), not out-diagnosing a frontier model. The one place the
  framework *won* (run 1, case-2) was where the base model actually erred — that
  is its real niche.
