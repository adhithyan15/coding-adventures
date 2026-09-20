## [Unreleased]

### Added — the joining column, from 0 of 11 to 10, and the repair language the book had none of (HL-C316)

- Add chapters 35-41 — "The Word for Not, and the Word for Sorry", "And, Or,
  But", "The Word That Carries a Thought", "Because, and Therefore", "If, and
  When", "He, She, We, and the One Who", and "Who, Where, When, How Many" — as
  seven five-lesson chapters, one new item per lesson, with the writing lesson
  third in every one. Gujarati: 34 chapters and 228 lessons -> 41 and 263.

**What the inventory measured, and what was actually wrong.** The A1 inventory's
`Jodaan` column stood at **0 of 11**. `ane` ("and") returned zero occurrences in
228 lesson files, so a learner could not say *tea and milk* although both words
are taught on facing pages of the same chapter. `nathi` returned zero too, so
"I don't understand" could not be said at all. `maaf` and `kshama` were both
absent, so there was no apology and no way to stop a stranger to ask anything.

**The finding under the finding: this was never a script debt.** Ten of the
eleven joining devices — *ane*, *athava*, *pan*, *ke*, *kemke*, *maate*,
*tethi*, *jo*, *jyaare*, *je* — are spelled entirely in glyphs the track had
already taught before chapter 30. Nobody had written the words down. The tranche
spends exactly **one** new letter in seven chapters: the **ફ** that *maaf karo*
required, and the apology is the only item in the run that needed one.

**Repair came first, because it was the starkest gap.** Chapter 35 teaches that
`nathi` REPLACES `chhe` rather than joining it; then *samajto nathi*, in the
speaker-gender form that matches the learner; then `ફ`; then *maaf karo*, which
both apologises and gets attention. A reader who is lost now has something to
say about it.

**Points closed in columns the tranche was not aiming at.** The negator alone
closed the can't-say-I-don't-understand function; the complementiser gave
`vicharvu` and `jaanvu` — both taught twenty-nine chapters ago — the first
objects they have ever had; and the question family closed four points at once.
Gujarati A1 coverage **100/210 -> 120/210 (48% -> 57%)**:

| column | before | after |
| --- | --- | --- |
| Jodaan (joining and subordination) | 0/11 | **10/11** |
| Prashna (asking questions) | 4/10 | 8/10 |
| Sarvanaam (pronouns) | 4/8 | 6/8 |
| Nakaar (negation) | 2/4 | 3/4 |
| Vyavahaar (communicative functions) | 17/30 | 20/30 |

Four spine omissions are realised as well: `COURTESY-SORRY`,
`CONNECTIVE-BECAUSE`, `QUESTION-WHERE` and — through chapter 36's carrier node —
the first segment `SPINE-DESCRIBE-QUALITIES` has ever held.

**Left uncovered, each with its reason written into the inventory.** The
distributive (`JOIN-04`) and `kevi rite` (`Q-06`) need no new glyph and are
one-lesson jobs. `neither ... nor` (`NEG-04`, `ADV-06`) needs the `na ... na`
correlative. `teo` (`PRON-04`) is the only one blocked by the script: it needs
the independent vowel **ઓ**, which no lesson teaches — so that point now names
its own fix as a script lesson rather than reading as missing vocabulary.

**Reinforcement, decomposed rather than reported.** Total misses 283 -> 364, and
a bare rise says nothing about whose debt it is, so it was measured against the
same corpus without chapters 35-41:

- **+41** the tranche's own atoms — the last chapters in the book, whose R3 and
  R4 windows fall past the final lesson and cannot be serviced from inside it.
- **+47** pre-existing atoms whose windows *did not exist* until the track grew.
  R4 is distance 80-250; at 228 lessons an atom introduced at position 151 had
  no R4 to miss, and at 263 it does. Nothing about those lessons changed.
- **-4** of those 47, closed deliberately: chapter 41's *where* lesson reads the
  chapter-22 row — *shaalaa*, *rasto*, the route map — cold, at distance ~106,
  which is inside R4 rather than decorative.
- **-3** pre-existing misses closed outright by the chapter-opening retrievals:
  `chhe`, `hun` and `kem` each reach their R4 for the first time.

R1 moved by only 4 across 38 new atoms, because every chapter opens by
retrieving the two preceding items by name — the boundary fix, built in from the
first chapter rather than added after the count rose.

**Two defects only the compiled page showed.** The `[YOU RECALL: …]` cue renders
raw, brackets and all, unless it is a list item — 239 corpus uses are bulleted
and the 28 bare ones were all mine; they are bulleted now and render as
*Recall: …* like every other cue. And the index's apparent dropping of the
`ક્ય`/`જ્ય` conjuncts is a `pdftotext` extraction artifact, not a rendering
defect: rasterising pages 231, 237 and 245 shows every conjunct set correctly,
and `missing_character` is 0.

### Verified

- Whole package suite: 124 test files, 1777 passed. Every `check:*` gate green.
- language-ladder: 39 files, 442 passed.
- The book compiles under XeLaTeX with `missing_character = 0`, every warning
  class at baseline, no undefined reference in the final pass, and three pages
  were rasterised and read.
- Ratchets held at zero: `scriptClosureViolations`, `neverTaughtGlyphs`,
  `glyphLessonSpikes`, `durationViolations`, **`forwardReferences`** (three
  previews I had written into chapters 37 and 39 named chapter-40 and chapter-41
  headwords and were removed before they could count).
- Taught glyphs 43 -> 44. Atoms taught 219 -> 257.

### Added — twenty new pre-A1 headwords, and the fifth-return slab that pays for them (HL-C286)

- Add chapters 31-34 — "The Table Is Set", "Inside the House", "Sun, Sky, and
  River", and "People, Paper, and a Book" — as four eight-lesson vocabulary
  chapters. Twenty new pre-A1 headwords arrive at **one new headword per
  lesson**: *bhāt*, *dāḷ*, *shāk*, *tel*, *kerī*; *bārṇũ*, *bārī*, *khurshī*,
  *chāvī*, *dīvo*; *sūraj*, *chandra*, *ākāsh*, *varsād*, *nadī*; *chhokro*,
  *chhokrī*, *māṇas*, *pustak*, *kāgaḷ*. Pre-A1 headwords move **52 -> 72**
  against the 300-word floor; the whole-track figure moves 71 -> 91.
- Keep every one of those twenty ear-first. Each arrives glossed and romanized in
  a pure-voice lesson before it is ever shown, each chapter closes on an oral
  checkpoint that scores listening and speaking separately, and exactly **one**
  word per chapter reaches the page — spelled entirely from signs taught before
  the first name exchange, so no new glyph lesson was needed or added.
- Add chapter 29, "The Time Words Reach the Page". This closes the second item
  HL-C271 left open: *savār*, *bapor*, *sānj*, *divas*, *mahino*, and *atyāre*
  were glossed and drivable but never written. All six now reach the page, one
  per lesson, on signs the reader has had since before chapter 8, then a payoff
  scores reading, speaking, and model-free writing apart and an R1 return writes
  the three the payoff did not ask for. It adds no headword and no glyph.
- Add chapter 30, "Fifth Return: The Core Verbs" — the slab HL-C271 filed rather
  than smuggled into a vocabulary chapter. Nine zero-new-atom lessons return the
  first five numbers and the fifteen core verbs of chapters 12-16 at distances of
  98 to 104 positions, each scoring recognition and model-free writing
  separately, and one of them closes the newly written time words at R2 and the
  village and shop at R3.

### Changed — measured continuity debt FELL while the track grew by half again

- Whole-track reinforcement misses move **339 -> 283** on a track that went 179
  -> 228 lessons. The previous tranche's rise was eligibility rather than
  neglect; this one pays that eligibility off. R4 misses alone move **101 -> 43**,
  and atoms never revisited at all move **5 -> 1**.
- Every new vocabulary chapter carries a *named distant band* of older material,
  chosen for the window a 228-lesson track has just made measurable and for
  whether the material actually belongs beside the new words: the food chapter
  returns *chā*, *dūdh* and *roṭlī* at R4, the house chapter the friend and
  family words, the sky chapter the eye/ear/mouth/nose words, and the people
  chapter the first three-place map plus hand and money. None of it is filler —
  a person walks to a market, a house holds a family, an eye looks at a sky.
- R3 misses rose 115 -> 117. The cause is stated rather than hidden: the last two
  chapters' own atoms sit too close to the end of the book for their third window
  to be serviced inside it. That residue is filed in `BACKLOG.d`.
- Pre-A1 atoms revisited fewer than twice move **24 -> 11**.

### Kept — the three script zeros, re-measured after the change

- Script-closure findings **0 -> 0**, never-taught glyphs **0 -> 0**, headwords
  without romanization **0 -> 0**. Every new headword was chosen so that its
  spelling uses only the 43 forms the book already teaches, which is why a
  twenty-word tranche cost no script debt at all.
- Duration violations stay at 0. Every new lesson is under the computed
  five-minute ceiling, and one that was not (`GU-R30-people-five-r1`, 349s) was
  split rather than re-declared.

### Changed — the ear-drivable share again

- Voice lessons move **34 -> 58** and the ear-drivable share **56% -> 65%**.
  Lessons reachable in chapter-prefix order without ever looking at anything move
  **88 -> 136**. Every lesson that needs a hand confines the handwriting to a
  single detachable `Writing —` block, so all 228 have a voice core; 91 are
  rescued for the hands-free view, up from 67.
- Register `ka` as a Gujarati sound tag. The allowlist carried `kha` but not the
  unaspirated `ka` that *shāk*, *kerī*, *pustak* and *kāgaḷ* all need.

### Added — Gujarati chapter 28 acquires eight time words by ear (HL-C271)

- Add chapter 28, "The Day and Its Times": fourteen lessons that teach eight new
  pre-A1 headwords — *savār*, *bapor*, *sānj*, *rāt*, *divas*, *mahino*, *āj*,
  and *atyāre* — one word per lesson, each glossed on first meeting and each
  usable by ear before anything is shown. Pre-A1 headwords move **44 -> 52**
  against the 300-word floor.
- Keep the chapter ear-first on purpose. Nine of the fourteen lessons are pure
  voice, and the five that need a hand confine the handwriting to a detachable
  `Writing —` block, so all fourteen have a voice core. The track's ear-drivable
  share moves **52% -> 56%**, and its drivable chapter prefixes **74 -> 88**.
- Write only two of the eight. *rāt* and *āj* reach the page because every sign
  in them was taught before chapter 8; the other six stay oral, which is the
  gloss-first-then-glyph order rather than an omission. No new glyph lesson was
  needed or added.
- Close the chapter on a four-skill payoff whose listening, speaking, reading,
  and writing scores stand separately, then return all eight one lesson later at
  R1. Every atom the chapter introduces is serviced inside the chapter.
- Add one distant return of the map words at R3. It closes the school and road
  meaning and script windows that the longer track newly made eligible, and it
  gives `GU-PERFORMANCE-MAP-TEN-FOUR-SKILL-01` its first revisit in the track's
  history — atoms never revisited fall **6 -> 5**.

### Fixed — every Gujarati headword is now sayable before it is readable (HL-C271)

- Declare `romanization` on the twenty-five native-script headwords that had
  none, from *ābhār* in chapter 2 to *rahevũ* in chapter 11. Under HL11 a
  headword with a romanization is exposure and one without it is something the
  reader has to decode, so this is not cosmetic: Gujarati headwords carrying no
  romanization move **25 -> 0**, and the corpus figure moves 283 -> 258. The
  romanization taken is the one each lesson's own title already printed, so no
  new claim about pronunciation is introduced.
- Move the three gender endings in *sārũ* out of Gujarati script and into
  romanization. The lesson sits in chapter 2 and was printing **-ો** and **-ી**,
  neither of which the book teaches until chapter 3; the reader was being asked
  to decode two signs nobody had taught. With that repaired and the headwords
  exempted, Gujarati script-closure findings move **2 -> 0**, and the track now
  has zero never-taught glyphs, zero closure findings, and zero unromanized
  headwords at once.
- The same edit replaced a "Look at that ending" with "Listen to that ending",
  which is both truer to a lesson about a sound and enough to make the lesson's
  core drivable.

### Changed — measured continuity debt rose while the track improved

- Whole-track reinforcement misses move **299 -> 339**, and the cause is
  eligibility rather than a skipped return. R4 opens 80 lessons after an atom
  appears, so a 165-lesson track could not measure past position 84; at 179 it
  reaches 98, and the thirty-six atoms of the core-verb chapters (13-16) became
  measurable in the same breath as they were found wanting. The pin in
  `tests/corpus/gujarati.test.ts` records the number and the reason; the
  fifth-return slab that clears it is filed as HL-C271 in `BACKLOG.d`.

### Added — Gujarati runway B closes measured R1/R2 windows (#13079)

- Return city writing inside the existing school lesson, two positions after
  its script introduction, without adding a new form or word.
- Add two zero-new-atom checkpoints that score listening, speaking, cold
  reading, and model-free writing separately. The first closes R1 for the
  three-word route performance; the second closes R2 for school and road
  meaning and script atoms. Every checkpoint stays under five minutes.
- Record the eleven windows newly made eligible by the longer runway in
  follow-up #13103; the measured total is 299 rather than a hidden green claim.

### Added — Gujarati doorway retrieval closes R4 (#12837)

- Add one zero-new-atom Chapter 19 checkpoint at track position 114, where all
  nine doorway forms are 88–80 lessons past introduction and inside R4.
- Use fresh shuffled orders and independent **9/9** thresholds for recognition
  and model-free writing, completing R1–R4 evidence for every form.

### Added — exact Gujarati R4 bridge D (#12860)

- Add five zero-new-atom Chapter 18 lessons at positions 109–113, returning the
  respectful wellbeing exchange and *we will meet* exactly 61 lessons after
  introduction.
- Keep recognition and model-free writing separate, and use the final farewell
  lesson to close four compatible R3 windows before the doorway R4 checkpoint.

### Added — exact Gujarati R4 bridge B (#12858)

- Add six zero-new-atom Chapter 16 lessons at positions 97–102. Name, my, the
  copula, the complete name frame, and the pleased-to-meet-you close each return
  exactly 61 lessons after introduction, with separate listening and model-free
  writing scores.
- Use the sixth position, whose matching source is a zero-atom checkpoint, to
  close measured R3 debt through the familiar wellbeing dialogue. No filler or
  new language is introduced.

### Added — exact Gujarati R4 bridge C (#12859)

- Add six zero-new-atom Chapter 17 lessons at positions 103–108. Familiar and
  respectful *you*, *what*, the respectful name question, the introduction
  exchange, *I*, and *how* each return exactly 61 lessons after introduction.
- Separate listening recognition from model-free Gujarati writing in every
  lesson, building gently from one-word contrasts to two familiar questions.

### Added — exact Gujarati R4 bridge A (#12857)

- Add six zero-new-atom, four-minute-or-shorter Chapter 15 lessons at track
  positions 91–96. The first five return short-u, chha, ka, retroflex nna, and
  sha exactly 61 lessons after introduction, closing each R4 window as it opens.
- Use the sixth position, whose matching source position is itself a zero-atom
  checkpoint, for independent reading and written production of the name
  exchange. Recognition and model-free writing keep separate mastery scores.

### Added — durable doorway retrieval closes R3 (#12835)

- Add one zero-new-atom, four-minute checkpoint after Chapter 13's meaning
  lessons, where all nine doorway forms are at the measured R3 distance.
- Shuffle reading and dictation independently so serial memory cannot replace
  recognition or model-free writing, and keep the two scores separate.

### Added — doorway retrieval earns R1 and R2 (#12834)

- Add two zero-new-atom micro-lessons at the exact expanding intervals: the
  final three doorway consonants return immediately for R1, then all nine forms
  return after the first five name-exchange lessons for R2.
- Score visible-form reading and model-free writing separately in both
  checkpoints. Each lesson stays under five minutes and a recognition answer
  cannot substitute for the written response.

### Added — nine prerequisite-safe doorway forms (#12811)

- Inserted a nine-lesson Chapter 3 before the first name exchange so learners
  meet **જ, ો, ં, ી, ુ, છ, ક, ણ, શ** one form at a time before later lessons
  ask them to decode those forms. Every micro-lesson stays at five minutes or
  less and follows observe/trace, guided copy, then delayed copy.
- Closed the chapter with separate reading and model-free writing checks over
  all nine forms. No new form is introduced in the payoff.
- Renumbered downstream chapters and generated book, narration, modality, and
  progress artifacts without changing stable lesson IDs or prerequisite links.
- Exact script-closure measurements improve from **25 to 16** never-taught
  glyphs and from **45 to 31** affected lessons, with no duration, order,
  forward-reference, glyph-load, or script-system regression.
- The longer runway exposes 21 additional mature reinforcement-window misses
  (**145 to 166**). This is recorded as follow-up #12814 rather than hidden by
  weakening the continuity gate.

### Fixed — the opening ramp now has two honest book chapters

- Split the 19-lesson opening container into an 11-step meaning-and-script
  chapter and an eight-step courtesy-and-response chapter. The learner still
  hears *namaste* before seeing its spelling, traces one sign at a time, and
  copies the whole word only after every load-bearing sign is familiar.
- Kept every canonical lesson at five minutes or less while giving each new
  chapter its own authored capability and payoff. The second chapter retains
  the full guided-copy, delayed-copy, and heard-word dictation runway for
  **હા**.
- Renumbered every downstream lesson chapter, capability entry, generated book
  target, and book include coherently through Chapter 13. Lesson IDs remain
  stable so prerequisites and durable review links do not break.
- Replaced every cross-chapter numeric pointer in lesson prose with a stable
  semantic pointer such as "the name exchange" or the named earlier lesson, so
  this split does not leave references that will rot at the next renumbering.
- Extended the authored session map through the already-shipped food, family,
  and body lessons (sessions 61–72), replacing a stale note that described
  those chapters as future work.
- The measured chapter-atom spike count falls **2 → 1** without changing lesson
  duration, learner order, script closure, or reinforcement counts. The one
  remaining chapter spike is preserved as explicit backlog rather than waived.

### Fixed — opening script prerequisites now precede the first read

- Turned the first *namaste* encounter into a meaning-first listening and
  speaking lesson. Its Gujarati headword is exposure with romanization; the
  body makes no decoding or copying demand.
- Moved the ten existing <=5-minute script micro-lessons out of the end-of-book
  Chapter 13 appendix and into the opening of Chapter 1 in authored sequence.
  Nine signs remain model-visible observe/trace work; the whole greeting is a
  supported guided copy only after every load-bearing sign is known.
- Removed the now-empty Chapter 13 book slot. This is a prerequisite-order
  repair, not a claim that Gujarati's remaining script or exam backlog is done.
- The measurable result is honest rather than cosmetic: load-bearing script
  violations fall **49 → 46**, forward-language uses **8 → 7**, and missed
  reinforcement windows **164 → 148**; the 32 never-taught signs remain future
  work. Consolidating 19 tiny lessons in Chapter 1 raises the aggregate
  chapter-atom spike count **1 → 2**, so #12452 records the prerequisite-safe
  learner-visible chapter split rather than hiding that debt.

### Added — Chapter 1 pre-A1 writing runway

- Moved the existing **હ** and **ા** piece lessons from the late script chapter
  into Chapter 1, immediately after spoken **હા / ના**. Each piece is traced
  with the model visible before the whole word is written.
- Added three bounded whole-word micro-lessons: one 90-second guided copy, one
  120-second delayed copy, and one 120-second heard-word dictation. The chapter
  payoff retrieves **હા** again without a visible model.
- Gujarati now proves all four cumulative pre-A1 writing stages: observe/trace,
  guided copy, delayed copy, and dictation/transcription. The track's writing
  gap falls by 28 cumulative track-level-stage pairs without claiming mock or
  exam readiness.
- Moving the two script atoms forward and retrieving them repeatedly improves
  the wider continuity-window debt by three. The three new whole-word steps
  introduce no atoms; their job is deliberate practice, not disguised content.

### Added — project-defined pre-A1 four-skill task shapes (HL18)

- Made the pre-A1 target executable as reading, listening, writing, and
  speaking papers with exact prompts, responses, timing, replay, aids, and
  scoring boundaries.
- Kept the assessment contract's point model literal: four separate 100-point
  papers, a 60-point floor on each, and no aggregate compensation.
- Writing assesses delayed recall, dictation/transcription, and bounded
  independent production. Tracing and visible copying remain gentle lesson
  supports and cannot earn exam credit.
- The inventory is not a readiness claim. Curriculum task coverage, two full
  mocks, rubrics, answer keys, calibration, and book-only human validation are
  still required.

### Added — pre-A1-to-C2 assessment contract (HL16)

- Added a clearly labelled project-defined Gujarati assessment ladder at
  pre-A1, A1, A2, B1, B2, C1, and C2 rather than implying an external
  qualification exists.
- Every rung requires independent reading, listening, writing, and speaking
  passes at 60%, with no stronger skill compensating for a weaker one.
- The contract carries writing from observe/trace and copying through delayed
  recall, dictation, connected composition, and timed exam production.
- Two timed mocks, rubrics, answer keys, calibration, and book-only human
  validation remain explicit dependencies, so the track cannot mistake a named
  destination for pass-readiness evidence.

### Fixed — three false forward-review claims

- `GU-C02-anand` now records the earlier name statement that its warm-up and
  knowledge directives actually rehearse.
- `GU-C04-kaale` now records its real `malishun` review instead of pointing
  ahead to the chapter's later assembled farewell.
- `GU-C05-kaam-karvun` no longer claims to review the following `rahevun`
  lesson; its exercises revisit `GU-C05-bolvun`.

The authored order and lesson durations do not change. Gujarati's three
order-integrity defects are now zero.

### Added — Chapter 13, the first nine pieces of the script (HL-C215)

Ten lessons. **Nine teach one piece each; one introduces nothing** and assembles
the greeting from pieces the reader can already write:

    હ  ા  આ  ન        ->  હા "yes", ના "no"
    મ  સ  ત  ે  ્     ->  નમસ્તે, including its conjunct

`scriptLessons` 0 → 10, `taughtGlyphs` 0 → 9, `neverTaughtGlyphs` **41 → 32**.

**The abugida is taught as a system, not as forty shapes.** Four ideas carry the
whole chapter, and each gets its own lesson: the **inherent vowel** (a bare
consonant already says *a*), the **mātrā** that replaces it, the **independent
vowel** that does the same job at the start of a word — a *different character*,
which is the commonest spelling error in the script — and the **virama**, which
deletes the inherent vowel and fuses two consonants into a conjunct.

After those four, every remaining letter is a shape rather than a rule. The
chapter says so explicitly: *the script has around forty more letters; it does not
have any more systems.*

**Two payoffs land inside the chapter.** One consonant and one mātrā make **હા**
readable at the second lesson; a third piece makes **ના** readable at the fourth.
The reader is decoding real words before the halfway point.

The headless-script fact — Gujarati is Devanagari with the top bar erased — is
taught on the very first shape, because it is the one difference visible at a
glance and it makes every later letter easier to place.


