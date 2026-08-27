## 0.9.2 — the SRS-weighted draw (HL03 phase 4, part 2 — quiz complete)

- Extends `src/quiz.ts` with the randomised cumulative quiz's draw:
  `pickNext(grid, states, session, rng)` selects a cell from the covered grid
  weighted by `cellWeight` — never-seen cells rank high, DUE cells rank higher
  the more overdue / lower-box / lapsed they are (the missed material review
  exists for), and not-yet-due cells sink to a floor so review stays
  interleaved. Per-cell Leitner state (`QuizState`, keyed by `cellKey`) reuses
  scheduler.ts's box/interval math. Deterministic via a seeded LCG (`makeRng`);
  the app never depends on `Math.random`.
- Two controls, both verified by injection: over many draws the sample spans
  MULTIPLE concepts AND languages (a collapsed draw fails); and the draw biases
  toward a missed/overdue cell over a mastered one by a wide margin (injecting
  uniform weighting fails it). This is the primary review mechanism from HL03 —
  "what is 5 in Telugu? 12 in Latin?" — now complete and pure.

