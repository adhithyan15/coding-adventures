## HL05 chapter capabilities — Chapters 4–6 and 19–33

- Extended `chapters.json` from 3 to 21 entries, covering every Spanish chapter
  that owns a `core/book-generation.json` target. Chapters 1–3 are untouched.
- Each new entry declares a first-person `canDo`, the shared spine nodes the
  chapter realises (derived from `curriculum.json` path segments), and a
  `payoff` naming the lesson that proves the claim, its kind, a one-line
  summary, and the knowledge atoms it exercises. Every `assesses` list is
  exactly the payoff lesson's own `practises.knowledge` — nothing invented.
- Payoff selection: Chapters 4–6 use their terminal `practice-mix`. Chapters
  19–33 have no practice lesson, so the payoff is the chapter's last lesson by
  sequence, which is where its recombination and wrap-up recall live.
- **Skipped, deliberately:** Chapters 7–18 (twelve chapters). Their lessons are
  still schema v1 with no declared `practises.knowledge`, so no payoff can be
  claimed honestly. They also own no book-generation target. The absence is
  tracked debt; a stub would have destroyed the HL05 gap report's signal.
- **Representativeness risks** measured against the 0.5 threshold in
  `core/chapter-policy.json` — payoff atoms over atoms the chapter introduces:
  Chapter 3 (0.25, 6/24, already known), Chapter 4 (0.32, 10/31), Chapter 5
  (0.29, 5/17), Chapter 6 (0.47, 9/19), Chapter 26 (0.38, 3/8). These are
  genuine split candidates — chapters that introduce far more than their payoff
  exercises — and are recorded rather than papered over by inflating `assesses`.
- Rewrote the ledger's `note` to describe what is authored, what is skipped, and
  why, replacing the placeholder that predated this work.

