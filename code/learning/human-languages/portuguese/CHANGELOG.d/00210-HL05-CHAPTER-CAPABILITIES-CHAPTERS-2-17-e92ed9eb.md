## HL05 chapter capabilities — Chapters 2–17

- Added `chapters.json`, the track's HL05 capability ledger, with 16 entries
  covering every Portuguese chapter that owns a `core/book-generation.json`
  target.
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
- **Representativeness risk** against the 0.5 threshold in
  `core/chapter-policy.json`: Chapter 2 scores 0.25 (4/16). It is the only
  chapter in the corpus with two `practice-mix` lessons — the full casual
  exchange at sequence 160 and the register drill `PT-C02-formal-practice` at
  170. The terminal-consolidation rule selects the register drill, which
  practises only four atoms; the earlier `PT-C02-practice` would score 0.94.
  Recorded rather than fixed by inflating `assesses` beyond what the terminal
  lesson actually practises. Every other chapter scores 0.50 or above
  (Chapter 16 at 0.50, Chapter 17 at 0.67, the rest 1.00).

