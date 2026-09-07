## Chapter capability ledger (HL05)

- Added `chapters.json`, the hand-authored capability ledger: one entry per
  chapter carrying a first-person `canDo`, the shared-spine nodes the chapter
  realises, and a `payoff` naming the lesson that proves the claim.
- Thirty of the thirty-three chapters are authored — Chapters 1, 2, and 6
  through 33. Titles and labels for Chapters 6–33 are copied exactly from
  `core/book-generation.json`; Chapters 1 and 2 have no generator target and
  take their names from the hand-authored `book/chapters/ch01`–`ch02` sources.
- Chapters 3, 4, and 5 are deliberately **absent**. Every lesson in them is
  still schema v1 with no `practises.knowledge`, so no payoff could name a real
  knowledge atom. The gap is recorded as debt in the file's own note rather
  than filled with a stub, because a stub would satisfy the gate while
  destroying the signal it exists to carry.
- Chapters 1 and 2 have no schema-v2 terminal consolidation lesson either:
  their `HI-C01-practice` and `HI-C02-practice` lessons are schema v1. Their
  payoffs therefore fall back to the chapter's last lesson by `sequence`, which
  in both cases is a Devanagari writing lesson — `HI-W02-ka-ta-mouth-order`
  and `HI-W05-write-namaste`. Both are recorded as `kind: task` and described
  as hand-writing work, not as spoken dialogue they are not.
- Every `payoff.assesses` list is exactly the payoff lesson's own declared
  `practises.knowledge`, so no chapter claims an atom its lesson never
  exercises.
- Four chapters will sit below the 0.5 representativeness threshold when the
  HL05 gates land: Chapter 1 (3/9), Chapter 2 (2/12), Chapter 6 (1/4), and
  Chapter 32 (1/3). Each is a chapter whose terminal lesson is narrower than
  the chapter as a whole; the fix is a real consolidation lesson, not a wider
  claim in this ledger.

