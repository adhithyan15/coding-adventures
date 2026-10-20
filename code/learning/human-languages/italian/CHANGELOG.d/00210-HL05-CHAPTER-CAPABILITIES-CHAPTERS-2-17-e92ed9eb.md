## HL05 chapter capabilities — Chapters 2–17

- Added `chapters.json`, the track's HL05 capability ledger, with 16 entries
  covering every Italian chapter that owns a `core/book-generation.json` target.
- Each entry declares a first-person `canDo`, the shared spine nodes the chapter
  realises (derived from `curriculum.json` path segments), and a `payoff` naming
  the lesson that proves the claim, its kind, a one-line summary, and the
  knowledge atoms it exercises. Every `assesses` list is exactly the payoff
  lesson's own `practises.knowledge` — nothing invented.
- Payoff selection: Chapters 2–5 use their terminal `practice-mix`. Chapters
  6–17 have no practice lesson, so the payoff is the chapter's last lesson by
  sequence, which is where its recombination and wrap-up recall live.
- **Skipped, deliberately:** Chapter 1. Its lessons are still schema v1 with no
  declared `practises.knowledge`, so no payoff can be claimed honestly, and it
  owns no book-generation target either. The absence is tracked debt; a stub
  would have destroyed the HL05 gap report's signal.
- **Representativeness** against the 0.5 threshold in
  `core/chapter-policy.json`: fifteen of sixteen chapters score 1.00, and
  Chapter 16 scores 0.67 (6/9). No Italian chapter falls below the threshold.

