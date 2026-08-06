# Changelog — Japanese track

All notable changes to the Japanese curriculum track are recorded here.

## [Unreleased]

### Added — Chapter 1, "Three Writing Systems in One Doorway" (HL-C40)

- Registered `japanese` in `core/languages.json` (Japonic / `japanese` script,
  bridging Chinese for the Sino-Japanese layer and Portuguese and German for the
  loanword layer) and declared the track's script in `track.json`, so no edit to
  the built-in `LANGUAGE_SCRIPT` map was needed.
- Added eight schema-v2 lessons, sequences 10–80, each with typed blocks, a
  first-line `hl-knowledge` directive per block, one compiled `hl-activity`, and
  a declared duration under 300 effective seconds:
  - `JA-C01-hai` — **はい**, hiragana and the mora as the unit of timing.
  - `JA-C01-iie` — **いいえ**, mora length as a meaning contrast (いえ vs いいえ).
  - `JA-C01-konnichiwa` — **こんにちは**, the moraic **ん**, and the topic
    particle **は** read *wa*, with the 1946 spelling reform as the reason.
  - `JA-C01-nihongo` — **日本語**, kanji, and the multiple-readings problem.
  - `JA-C01-koohii` — **コーヒー**, katakana, the chōonpu, and the Arabic *qahwa*
    borrowing shared with English *coffee*.
  - `JA-C01-arigatou` — **ありがとう**, the dakuten, and 有り難し "hard to exist".
  - `JA-C01-gozaimasu` — **ありがとうございます**, politeness as verb morphology.
  - `JA-C01-practice` — the six-line doorway exchange payoff.
- Added `curriculum.json` with three path segments, a ledger entry for all eleven
  spine nodes, and seven Japanese-specific extension nodes (five `script`, one
  `register`, one `consolidation`).
- Added `chapters.json` with the chapter capability and a payoff assessing ten of
  the chapter's eighteen introduced atoms.
- Added `roadmap.md`, `session-map.md` (review ledger through S23),
  `pronunciation-reference.md`, and `README.md`.
- Added `data/scripts/japanese.json`: one inventory covering hiragana, katakana,
  the length bar, and the seven kanji the lessons use, with `role` distinguishing
  the systems and the dakuten/handakuten as marks.
- Vendored `_fonts/NotoSansJP-Subset.ttf` (SIL OFL 1.1) with `_fonts/subset-jp.sh`
  to regenerate it, following the existing `subset-cjk.sh` precedent.
- Added the generated LaTeX Chapter 1 and the book scaffolding, with a
  `japanese-main` script set mapping Katakana, Hiragana, and Han to one `\ja`
  command.

### Notes on method

- Seven of the eight lessons carry a `script` block and are therefore derived as
  `sight` under HL08. That is deliberate: a sign's shape cannot be read aloud, and
  marking these lessons drivable would promise a learner something no narration
  can deliver. The chapter's drivable prefix is 0 and its practice lesson is the
  only `voice` lesson.
- No cognate with English is claimed anywhere except **コーヒー**, where the
  shared Arabic source is real. `JA-C01-hai` states plainly that its own
  etymology is unsettled and that no English cousin exists.
