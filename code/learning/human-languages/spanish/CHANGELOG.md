# Changelog

## Part I complete (Chapters 2-4)

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

## Part 0 & Chapter 1 (book/framework expansion)

- Added five standing pillars to `HL00`: a **Grammar Lens** unit section
  (plain-language grammar concept + English contrast), a **`morphology`**
  unit type (lexical Latin roots), a **Part 0** phase (script/sound-system
  introduction, scaled per language), a **Grammatical Gender** methodology
  (nouns tagged from the first one onward), and a **LaTeX book** deliverable
  per language track (CC BY-SA 4.0, XeLaTeX/fontspec/polyglossia).
- Renamed Phase→Part, Week→Chapter in all user-facing prose (`roadmap.md`,
  READMEs); frontmatter fields (`phase`, `week`) unchanged internally.
- Added Part 0 — Sounds & Letters (`units/ES-P0-U00A/B/C`): the five vowel
  sounds, consonants that differ from English, stress & written-accent
  rules. Renumbered Chapter 1's session schedule (+3) to make room.
- Added a morphology unit (`ES-P0-M01`): the Latin *clamare* root ("to
  call/shout") — extends the *llamar* etymology from Unit 1 into
  *llamar/llamada/exclamar* (Spanish) and *claim/exclaim/acclaim/clamor/
  proclamation* (English).
- Retrofitted Grammar Lens sections into `U01` (reflexive-verb preview),
  `U02` (pronouns, pro-drop contrast), `U03` (linking-verb concept), `U05`
  (stative vs. dynamic), `U06` (full ser/estar contrastive payoff); added a
  grammatical-gender explanation to `U07` (days of the week, all masculine).
- Rewrote `roadmap.md` with Part/Chapter framing and a "Part 0" lead-in;
  elevated Chapter 3's description to gender-mastery framing.
- Rewrote `session-map.md` for the new 10-session Part 0 + Chapter 1
  schedule.
- Added `book/`: a LaTeX book (`book.tex`, `preamble.tex`,
  `chapters/part0-sounds-and-letters.tex`,
  `chapters/part1-ch01-greetings-and-pronouns.tex`), compiled and verified
  with XeLaTeX via `latexmk`. Title page, preface, and CC BY-SA 4.0 notice
  included. Grows one chapter at a time from here.

## Chapter 1 (originally "Week 1", Phase 0 — Foundations)

- Added `roadmap.md`: full year skeleton, Phases 0-4 plus buffer/assessment weeks.
- Added `session-map.md`: Week 1 session composition (sessions 1-7) and worked spaced-repetition schedule (N+1/N+3/N+7/N+15).
- Added Week 1 units (`units/ES-P0-U01` through `ES-P0-U08`): greetings, subject pronouns, *ser* (identity/origin), numbers 0-10, *estar* (state/location), *ser* vs *estar* contrast (practice-mix), days of the week, and an "introduce yourself" capstone (practice-mix) recombining the whole week.
- Added two worked `review` units (`ES-P0-R01`, `ES-P0-R02`) as concrete examples of the "fresh combination, not verbatim repeat" review pattern; later review instances follow the same pattern and are described algorithmically in `session-map.md` rather than each hand-authored.
- Etymology notes on every new vocabulary item, with a deliberate first look at Spanish's Arabic-derived vocabulary (*cero* ← Arabic *ṣifr*, also the source of English *cipher*) alongside the primary Latin chain.
