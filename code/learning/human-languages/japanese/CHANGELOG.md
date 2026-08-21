# Changelog — Japanese track

All notable changes to the Japanese curriculum track are recorded here.

## [Unreleased]

### Added — cumulative pre-A1 writing evidence (#12365)

- Turned four already gentle Chapter 2 lessons into an explicit cumulative
  writing ladder: trace one visible sign, copy one visible sign with cues, hide
  and recall one two-sign word, then transcribe one heard or romanized mora.
- Kept every action inside its original one-sign load and below five minutes;
  the evidence metadata now follows the learner action instead of merely tagging
  lessons that happen to contain handwriting.

### Added — pre-A1 four-skill task shapes (#12363)

- Made the project-defined rung below JLPT/JF A1 executable as four independently
  scored reading, listening, writing, and speaking sections.
- Kept writing productive: delayed kana recall, dictation/transcription, and one
  bounded independent response earn points; tracing and visible copying do not.
- Recorded exact project-owned timing and length bounds without inventing an
  external “JLPT N6” or implying that chapter coverage proves readiness.

### Added — pre-A1-to-C2 four-skill assessment contract (#12361)

- Replaced the old unofficial one-level-per-JLPT mapping with the official CEFR
  reference score bands introduced on JLPT score reports in December 2025.
- Added a seven-rung assessment target that preserves the official JLPT
  language-knowledge, reading, and listening pass conditions where they apply,
  then adds independently scored JF Standard/CEFR-aligned writing and speaking.
- Kept pre-A1 and C2 explicitly project-defined: JLPT's official CEFR reference
  range begins at A1 and ends at C1, and JLPT itself tests no production or
  interaction at any level.

### Added — Chapter 2, eight hiragana signs, one per lesson (HL-C211)

Ten lessons. **Eight teach one sign each; two introduce nothing at all** and
instead assemble a word out of signs the reader can already write:

    i -> ha -> [hai] -> e        both yes-or-no answers become readable
    ko -> n -> ni -> chi -> wa -> [konnichiwa]

`scriptLessons` 0 -> 10, `taughtGlyphs` 0 -> 8, `neverTaughtGlyphs` **43 -> 35**.
Corpus-wide `tracksTeachingNoScript` falls to **6**.

**The sign for *wa* is taught deliberately late**, after the greeting is already
known — so that the shape arrives with its warning attached. The daytime greeting
*sounds* like it ends in *wa* and is written with the sign read *ha*, because that
sign is doing a second job as the topic marker. Teaching the *wa* sign first would
have quietly created the commonest beginner spelling error in the language.

**Three signs deliver a payoff the same day they are learned.** Two signs make the
word for *yes* readable; a third adds the word for *no*. The assembly lessons exist
to mark that moment — the point where a reader stops recalling a shape and starts
sounding one out.

**The mora rule gets its clearest evidence here.** The sign with no vowel takes a
full beat, exactly as long as the four around it, which is why the greeting is five
beats rather than four. The romanization cannot show that; the signs can.

This is the first of five tranches for this track. 35 glyphs remain — 16 hiragana,
10 katakana, 9 kanji — and the katakana and kanji sets are separate writing systems
with their own ramps.


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
