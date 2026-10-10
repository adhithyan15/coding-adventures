---
category: Testing & coverage
---

# A test that proves a measure is not vacuous by reading debt off the real corpus goes red when the debt is paid

`tests/script-closure.test.ts` guarded "closure and the pace budget are not
merely agreeing because both are empty" with
`expect(measureScriptRamp(realCorpus).summary.lessonViolations).toBeGreaterThan(0)`.
The glyph-step burn-down brought the real corpus to zero over-budget lessons,
and the test went red because the debt was PAID. The same file had already
been bitten twice by the same shape (`toBeGreaterThan(500)` on closure debt,
then a "closure finds five times what pace finds" comparison).

Fix: demonstrate "this measure can fire" on a tiny synthetic input that must
trip it (one lesson showing four new shapes -> exactly one violation), and keep
the real corpus under a ceiling or a zero gate.

Rule: never assert that the real corpus still HAS a defect. A non-vacuity
check belongs on a fixture; the corpus only gets ceilings that may fall.
