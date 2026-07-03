# ADJ101 — medicine, Haiku vs Haiku, WITH programs: FINDINGS

**Status: COMPLETE.** 8 multi-step clinical-calculation items (Cockcroft-Gault CrCl, 4-2-1 maintenance
fluids, weight-based dosing, BMI, anion gap, corrected calcium, heparin units/h, reimbursement). Same
model, **Haiku**, both arms: **BARE** (does the math in-head, prose) vs **FRAMEWORK** (Haiku does ONLY
extraction → emits a program over provenanced facts; the executor runs it). Gold tool-derived.

## Headline

| arm | correct | **auditable** |
|---|---|---|
| bare-Haiku (in-head) | 7/7 well-formed¹ | **0/8** |
| **framework-Haiku (program)** | **8/8** | **8/8** |

¹ Two scoring artifacts were *mine*, not the model's: MED-FLUID's bare answer was "66 mL/h" (the
auto-extractor grabbed the "26 kg" first), and MED-REIMB's bare prompt lost its `question` field on a
paste. On the well-formed items, bare-Haiku computed every clinical formula **correctly in-head**.

## The honest result (consistent with the cross-domain head-to-head)

1. **Accuracy parity.** Modern Haiku is *good* at these common clinical calculations in its forward
   pass — it got Cockcroft-Gault (66.7), the 4-2-1 rule (66), corrected calcium (8.8), anion gap (18),
   BMI (29.39), weight-based dose (120), and the heparin rate (1200) right. The framework does **not**
   make Haiku *more accurate* on formulas it already knows. (This is the FrugalGPT axis, not ours.)

2. **The framework's win is auditability + correctability — 8/8 vs 0/8.** Every framework answer is a
   **program over typed, provenanced facts**, so the *formula itself is exposed and checkable*:
   - Cockcroft-Gault's `140` and `72` are facts (inferred-ENTAILED from the formula given), not
     literals recalled from memory;
   - corrected-Ca's `0.8` correction factor is a **stated** fact tied to the span "0.8 mg/dL for each
     1 g/dL" — you can see exactly which factor the computation used;
   - the 4-2-1 bands (`10/20`, `4/2/1`) are facts, and the molecular/dosing arithmetic runs in code, not
     in the model's head;
   - derived formulas (anion gap, albumin deficit, heparin concentration ratio) are flagged as
     **surfaced assumptions** — the audit pointing a reviewer at exactly the steps to verify.
   Bare's 8 answers are prose: correct here, but the formula constants are **invisible recalls** a
   clinician cannot inspect, and nothing is machine-checkable or one-edit-correctable.

3. **The latent-risk point (why this matters for deployment).** Bare-Haiku happened to recall every
   constant correctly. But that risk is **invisible** in prose — on a less-common formula (or a
   misremembered constant: a `0.7` Ca factor, a wrong CrCl denominator), bare would be **silently
   wrong**. The framework makes the formula choice **auditable before it bites** and **correctable by
   editing one fact** (override → re-derive, zero model calls). For a high-stakes domain like medicine,
   *that* is the deployable property — not the (already-good) accuracy.

## Bottom line
With programs written, a **weak model's clinical computation becomes auditable and correctable at
accuracy parity** with in-head: 8/8 framework answers are inspectable programs over provenanced facts
(every formula constant sourced), vs 0/8 for bare prose. The framework doesn't make Haiku a better
calculator; it makes Haiku's calculations **defensible** — which is the whole thesis, now shown in the
domain where it matters most.

## Reproduce
Framework: `pilot10/translate10.workflow.js` with `{model:'haiku', items}`; bare:
`bare_arm/bare_solve.workflow.js` with `{model:'haiku', items}`; execute via
`../provenance_program.py`. Items + gold: `items_med8.json`; saved runs: `fw_haiku_emissions.json`,
`bare_haiku_results.json`.
