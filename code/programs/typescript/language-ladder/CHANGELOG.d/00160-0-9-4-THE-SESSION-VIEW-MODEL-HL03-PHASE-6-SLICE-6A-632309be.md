## 0.9.4 — the session view-model (HL03 phase 6, slice 6a)

- `src/sessionplan.ts` — the seam that assembles the four engine modules into
  one session, with no DOM (the UI slice renders what this returns):
  `planSession(current, covered, lessons, activeCount)` returns the **teaching
  pass** (the current concept swept across the active chain, with connections)
  and the **review pass** (the covered grid the quiz draws from).
- `applyAnswer(progress, cell, correct, session, chosenKey?)` threads the state
  that makes review adaptive: a hit **promotes** the cell (comes back later), a
  miss **demotes** it (box 0, due now) and logs the confusion — so the next
  `pickNext` leans on what was just missed. Immutable; `initProgress` seeds it.
- Controls bite (fault-injected): a review pass that only covered the current
  concept fails the "spans every covered concept" test; a no-op `applyAnswer`
  fails the "missed cell outweighs a mastered one" test. Verified against the
  real curriculum (COURTESY-THANKS teaches across all ten, reviews alongside
  GREETING-HELLO). Pure, deterministic. Next: slice 6b renders it.

