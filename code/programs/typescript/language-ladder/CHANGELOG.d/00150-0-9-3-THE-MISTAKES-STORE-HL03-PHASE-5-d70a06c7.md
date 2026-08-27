## 0.9.3 — the mistakes store (HL03 phase 5)

- `src/mistakes.ts` — records each quiz answer and, crucially, WHAT THE LEARNER
  CHOSE when wrong (the confusion — e.g. picking the French cognate's meaning
  for the Spanish word). `recordAnswer` appends immutably; `demote` feeds a miss
  back into the SRS (box→0, due now, lapse++) so the item resurfaces sooner in
  `pickNext`; `confusions` rolls the wrong answers into ranked "what you keep
  mixing up" pairs.
- Grounded: a confusion only ever appears if the learner actually made it — no
  pair is inferred. Pair keys use `JSON.stringify`, not delimiter-joining, so an
  id containing a comma can't collapse two distinct confusions into one.
- Controls bite (fault-injected): a no-op demote fails the "missed cell
  resurfaces" test (its draw weight must jump above a mastered cell's); a
  fabricated pair fails the "nothing invented" control. Pure, deterministic, no
  I/O — the caller passes the session index in.
- This completes the pure-logic layers of HL03 (phases 2–5). Next: phase 6, the
  UI that unifies the four modes into one curriculum-driven session.

