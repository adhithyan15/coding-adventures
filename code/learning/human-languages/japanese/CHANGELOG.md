# Changelog — Japanese track

All notable changes to the Japanese curriculum track are recorded here.

## [Unreleased]

### Added — chapters 14 and 15: the cardinals one to ten, and the two signs they cost

`JA-A1-NUM-01` was ticked on ONE numeral, and that one was inside a phrase. The
track taught no other number and no counter. Fourteen lessons in two chapters
close it on all ten, and the coverage total does not move — which is the finding,
not an oversight: the tranche DEEPENS two existing ticks rather than adding one,
and a coverage column cannot see that. The named test pin exists for exactly that
reason.

**TWO OF THE TEN WERE ALREADY IN THE READER'S HANDS, AND THE CHAPTERS OPEN ON
THEM.** *ichi* has been said since chapter 9 inside **もういちど**, whose lesson
states in so many words that *ichi* is one. And **五** has been in the hand since
chapter 5, where the script lesson that taught its four strokes wrote on the page
that **the sign means five on its own** and then asked for nothing but its sound,
because it was there to cue the *go* inside **語**. That was an explicit deferral,
and chapter 14 is where it is paid. So the chapter opens **ichi, go** — one out of
the mouth, one out of the hand — and neither lesson spends a sign.

**THE ORDER IS BY COST, NOT BY NUMBER, IN BOTH CHAPTERS.** Chapter 14 runs
*ichi, go, ni, san, yon* and costs **zero** new signs. Chapter 15 runs *nana,
hachi, ku*, then buys **ろ** for *roku* and **ゅ** for *jū* — so the reader counts
round an audible hole at six for three lessons, and the chapter says on the page
why the hole is there. Six and ten are the only two numerals between one and ten
whose everyday reading needs a hiragana sign this book had not taught.

**THE GRAMMAR ATOM IS THE SEAM THE COUNTERS WILL RUN ALONG, AND IT IS TAUGHT WITH
ITS EDGES.** Wiktionary's readings box gives **し** as the on'yomi of four and
**よん** as the kun'yomi, and **しち**/**なな** the same way at seven. So at four
and seven the NATIVE word stands inside a borrowed count and is the one the reader
will hear. `JA-GRAMMAR-KUN-IN-THE-COUNT` is introduced at four and held at seven,
and then the chapter shows both edges rather than leaving the reader to overgeneralise:
**はち** has no second name at all, and **く** has one whose second reading the
same box calls on'yomi as well. Some numbers have two names; only some of those
get the second from the native side.

**TEN WORDS, NINETY-NINE NUMBERS.** `JA-GRAMMAR-JUU-COMPOUND` is what carries the
point past ten: a numeral before **じゅう** multiplies it, one after it is added,
and nothing is inserted between them.

**REINFORCEMENT: THE TRACK'S PERFECT RECORD IS INTACT, AND ONE DEFECT IS GONE.**

    reinforcementWindowMisses          0 ->   0
    reinforcementMissesByWindow-R1     0 ->   0
    reinforcementMissesByWindow-R2     0 ->   0
    reinforcementMissesByWindow-R3     0 ->   0
    reinforcementMissesByWindow-R4     0 ->   0
    atomsTaught                      111 -> 125
    atomsNeverRevisited                1 ->   0
    scriptClosureViolations            0 ->   0
    neverTaughtGlyphs                  0 ->   0

Fourteen more lessons make **29** (atom, window) slots newly judgeable on OLDER
atoms — one R1, two R2, ten R3, sixteen R4 — debt the length EXPOSES rather than
creates. Every one is answered, and answered on a **diagonal**: lesson *n* of the
tranche services the R4 of the atom introduced at old position 36+*n* and the
R3 of the atom at 98+*n*, so the fourteen warm-ups walk the whole of chapters 8-9
and the whole of chapter 13 in order. The tranche's own fourteen atoms create
**zero** debt. Zero regressions on previously judged slots. And
`JA-PERFORMANCE-FAMILY-NINE-01`, introduced by the track's last lesson and
therefore never revisited, is revisited now — which is why `atomsNeverRevisited`
falls to **zero**.

**THE SCRIPT COST WAS TWO SIGNS AND IT WAS MEASURED, NOT ASSUMED.** ろ and ゅ join
`data/scripts/japanese.d`, with owner declarations and evidence. **ろ's stroke
order was observed**: the cited Sirgazil animation was fetched, expanded to its 26
frames and read, and the start marker never leaves the upper-left origin, so the
run is single and `penLifts` is 0. **ゅ claims no independent handwriting
evidence**: it reuses ゆ's observed two-run movement and says so in its
`variation` field, exactly as U+3063 small tsu reuses つ's. Closure held at zero
only because the chapters do NOT print the nine kanji they cannot teach — three
Wiktionary link texts originally carried 四, 七 and 九 and were rewritten, which
the closure measurement caught and prose review would not have.

**A LEVEL GATE FLIPPED, AND IT IS WORTH THE SAME CAVEAT CHINESE AND MARWADI GOT.**
`JA-C14-yon` realizes `SPINE-COUNT-ONE-TO-FIVE`, so the track's `reach` moves from
pre-A1 to A1 and no track in the corpus is now below A1. Everything else the track
holds is still pre-A1 and `attained` has not moved. One node realized is not a
level reached.

**NOT CLAIMED, and the inventory notes now say so rather than leaving it to be
inferred.** The nine remaining kanji (五 is the only one of the ten the reader can
write as Japanese writes it). Zero, which needs **れ**. *kyū*, which needs **き**
and is given in romanization with the reason. The COUNTERS — `JA-A1-NUM-02` —
which is the next tranche and is what still stops the reader counting *things*
rather than counting aloud; the reader already owns one counter without its being
named as one, the **-ど** of chapter 9's **いちど**, whose lesson says *do* counts
an occurrence. And ordinals, `JA-A1-NUM-03`, which are blocked behind the
counters and not behind the cardinals any more.

### Changed — learner guide follows the authored runway (#12568)

- Replaced the obsolete single-chapter/eight-lesson description with the current
  twelve-chapter, 100-session pre-A1 sequence.
- Reported vocabulary and script coverage from the canonical lesson ledger and
  script-closure measurement: 28 word lessons, seven phrase lessons, and 47 of
  47 shown glyphs taught with no closure violations.
- Made the five-minute script-before-decoding and writing-stage policies
  explicit, and connected the opening book honestly to the full pre-A1-to-C2
  non-compensatory assessment contract.

### Added — one sound-first farewell (#12475)

- Added six <=5-minute sessions for **さようなら**: sound and social job first,
  then only the three new signs **よ**, **な**, and **ら**, a sign-by-sign
  assembly, and a four-skill payoff.
- Reused known **さ** and **う** instead of reteaching them, while preserving a
  trace-copy-recall writing ramp for the genuinely new shapes.
- Realized the shared `FAREWELL` function without pretending this one expression
  covers every relationship or departure context.
- Grounded the A1 headword and contextual warning in Japan Foundation Marugoto
  and Irodori materials.

### Changed — script closure before decoding (#12471)

- Reordered the 29 existing lessons so every hiragana writing step precedes the
  word that asks the learner to decode it; the old content-first order is gone.
- Split the starter into seven small chapters, keeping hiragana, kanji, and
  katakana arrivals separate and moving the four-skill doorway exchange to the
  end of the runway.
- Added nine <=5-minute writing lessons: `日`, `本`, three components and the
  assembled `語`, followed by `コ`, the long-vowel mark `ー`, and `ヒ`.
- Rewrote advanced etymology and keigo examples in romanization so fifteen rare
  or unrelated signs no longer become accidental pre-A1 decoding tests.
- Replaced the stale eight-session map with the complete 38-session authored
  order and its machine-checked review rule.

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
