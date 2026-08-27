## 0.8.0 — the language chain and the teaching sweep (HL03 phase 2)

- First implementation piece of the unified language-learning app
  ([HL03](../../../specs/HL03-unified-language-learning-app.md)): a pure
  `sequence.ts` module encoding the fixed **language chain**
  (Spanish → Latin → French → German → Arabic → Hindi → Tamil → Kannada →
  Telugu → Malayalam) and the **teaching sweep** — for one concept, the active
  languages that teach it, walked in chain order.
- `teachingSweep(concept, lessons, active)` filters to the concept, restricts to
  the active chain prefix, skips languages that do not teach it, and orders the
  result by the chain (never by input order). `sweepableConcepts` lists concepts
  in book order (earliest chapter first). No UI — sequencing logic only.
- Verified against the real curriculum: `GREETING-HELLO` sweeps all ten
  languages in exact chain order. Every honesty check is paired with a control
  that fails on broken input — and writing this caught a redundant active-filter
  that would have made the "only active languages" test vacuous; removed so the
  test can actually fail.

