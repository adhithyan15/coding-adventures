## Part I complete (Chapters 2-4) — pre-redesign, superseded

- Added Chapter 2 (`units/ES-P0-U09` through `U15`): numbers 11-100
  (including the "diez y seis"→"dieciséis" fusion and its English
  "sixteen" parallel), telling time (*es la una* vs. *son las dos*),
  months & seasons, the seven core question words (and their written-accent
  rule), survival phrases, and a practice-mix capstone.
- Added Chapter 3, the promised gender-mastery chapter (`units/ES-P0-U16`
  through `U21`): articles, the *-o*/*-a* pattern's real exceptions (*el
  día, la mano, el problema, el agua*), adjective agreement, colors
  (including *azul*, another Arabic loanword alongside *cero*), family
  vocabulary (the one place grammatical gender lines up with real-world
  sex), and a practice-mix capstone.
- Added Chapter 4, closing Part I (`units/ES-P0-U22` through `U26`): *hay*,
  *tener* (a first stem-changing verb), possessive adjectives, negation
  (Spanish's mandatory double-negative, a genuine "opposite of the English
  rule" case), and a cumulative Part I review capstone.
- Extended `session-map.md` through session 28 (Part I's end); introduced
  the bonus-queue framing explicitly once review volume exceeds what a
  2-4-item core block can hold, rather than continuing to hand-track every
  single far-future review individually.
- Extended `book/` with `chapters/part1-ch02-numbers-and-time.tex`,
  `part1-ch03-gender-mastery.tex`, `part1-ch04-hay-tener-negation.tex` —
  Part I is now fully typeset, title page through Chapter 4.
- Updated `HL00` with four standing-methodology amendments driven directly
  by learner feedback: just-in-time **script** introduction (no dedicated
  alphabet-review chapter for any track that needs one), the same
  just-in-time principle extended to **grammar** (motivated by the new
  Tamil track), a **frequency-driven content selection** principle, and the
  **Cross-Language Comparison Web** — an accumulating hierarchy where each
  new language compares against every language already established before
  it (Spanish→English/Latin; French→+Spanish; German→+French; Arabic→+German;
  Hindi→+Arabic+Sanskrit; Tamil→+Hindi+Sanskrit; Kannada/Malayalam/Telugu→all
  of the above).
- Added `.github/workflows/human-languages-books.yml`: CI that discovers
  every language's `book/` directory and compiles it to PDF with XeLaTeX,
  uploaded as a build artifact per language.
- Bootstrapped seven more language tracks (French, German, Arabic, Hindi,
  Tamil, Kannada, Malayalam, Telugu) — see each track's own `CHANGELOG.md`.

