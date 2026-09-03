# Changelog

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


## Pre-A1 vocabulary tranche — twelve everyday nouns, three chapters

The level gate (`src/level-gate.ts`) reports every track blocked on
**vocabulary**: 300 distinct headwords at or below pre-A1. This tranche
authors twelve concrete nouns across three new chapters, continuing the
Hindi/Arabic/Tamil/German program and confirming the same mechanism:
`vocabularyOf()` counts distinct `headword:` strings, so twelve one-headword
lessons move Gujarati's pre-A1 vocabulary by exactly twelve — **22 → 34**
distinct headwords at or below pre-A1 (against the 300 target, shortfall
278 → 266), and 41 → 53 distinct headwords track-wide. No bulk credit;
measured, not assumed.

| Lesson | Concept | Word |
|---|---|---|
| `GU-C10-paani` | `GU-FOOD-WATER` | પાણી |
| `GU-C10-chaa` | `GU-FOOD-TEA` | ચા |
| `GU-C10-dudh` | `GU-FOOD-MILK` | દૂધ |
| `GU-C10-rotli` | `GU-FOOD-BREAD` | રોટલી |
| `GU-C11-mitra` | `GU-PEOPLE-FRIEND` | મિત્ર |
| `GU-C11-kutumb` | `GU-FAMILY-WHOLE` | કુટુંબ |
| `GU-C11-bhai` | `GU-FAMILY-BROTHER` | ભાઈ |
| `GU-C11-bahen` | `GU-FAMILY-SISTER` | બહેન |
| `GU-C12-aankh` | `GU-BODY-EYE` | આંખ |
| `GU-C12-kaan` | `GU-BODY-EAR` | કાન |
| `GU-C12-modhu` | `GU-BODY-MOUTH` | મોઢું |
| `GU-C12-naak` | `GU-BODY-NOSE` | નાક |

**Atom-first, gender taught with every noun.** Each lesson introduces 2–3
knowledge atoms (`GU-LEX-*`, usually `GU-ETYMON-*` and sometimes a
`GU-GRAMMAR-*` or `GU-HISTORY-*`), at or under `maxNewAtomsPerLesson: 3`.
Chapter 10 introduces 11 atoms, chapter 11 introduces 9, chapter 12 introduces
10 — all at or under `maxNewAtomsPerChapter: 12`. Every noun's gender is
stated with the noun.

**Chapter 10 — Water, Tea, Milk, and Bread** (`SPINE-POLITE-REQUEST-REPAIR`,
previously the one pre-A1 spine node with zero lessons on it): the chapter
builds Gujarati's first polite-request pattern from scratch — **[item],
મહેરબાની કરીને** ("[item], please," literally "having done a kindness"),
using the `કરીને`/`કરવી` family already established by `મદદ કરવી`. **પાણી**
and **દૂધ** are inherited straight from Sanskrit with confirmed PIE roots
(*peh₃(i)-* "to drink" → English *potion*, *poison*, *symposium*; *dʰewgʰ-*
"to be fit, useful" → English **doughty**, not milk-related at all in
English). **ચા** is a loan that took the **overland** route out of Mandarin
Chinese by way of Persian — the same route as Hindi/Russian *chai* and
Turkish *çay* — as opposed to English *tea*'s **sea** route via Hokkien
Chinese and Dutch traders; ચા is also one of Gujarati's few nouns whose
gender is not fixed (masculine or feminine). **રોટલી** is inherited too, but
Turner's and Mayrhofer's comparative dictionaries leave its own root an open
question — an honest dead end, not an invented one. The payoff also uses
**પાણી**'s neuter gender (despite its **-ī** ending) to teach that
**રોટલી**'s own feminine **-ī** is a hint, not a rule.

**Chapter 11 — Friend and Family** (`SPINE-EXCHANGE-NAMES`): **મિત્ર**
("friend") is Sanskrit, from Proto-Indo-Iranian *\*mitras*, "(that which)
causes binding" — the same word as Avestan Mithra, whose Middle Persian
descendant *mihr* is the very root **મહેરબાની** was built on last chapter, so
the two words turn out to be cousins three chapters apart. **કુટુંબ**
("family") is a **learned borrowing** (*tatsama*) from Sanskrit rather than a
word worn down through sound change (*tadbhava*) like the other three —
and Sanskrit's own *kuṭumba* is itself thought to be a **Dravidian** loan,
compared to Tamil *kuṭimai*. **ભાઈ** ("brother") is English *brother*'s
unbroken cousin, down to the shared PIE root *bʰréh₂tēr*. **બહેન**
("sister") is deliberately **not** *bhāī*'s parallel: it is not from the PIE
root behind English *sister* (*swésōr*) at all, but from Sanskrit *bhaginī*,
disputedly built on *bhaga*, "a share, good fortune" — named as disputed
because scholarly opinion is genuinely split, not settled for convenience.

**Chapter 12 — Eye, Ear, Mouth, and Nose** (`SPINE-CHECK-WELLBEING`):
**આંખ**/*eye* and **નાક**/*nose* both trace to confirmed PIE roots shared
unbroken with their English counterparts (*h₃ekʷ-*, *nehas-*). **કાન**/*ear*
is inherited from Sanskrit *karṇa* with no agreed root beyond it — scholars
have proposed "defect," "point," "handle," and a link to "to hear" without
settling — and its gender shifted from Sanskrit masculine to Gujarati
feminine, while **મિત્ર** kept its Sanskrit masculine, making the point that
inheritance does not guarantee gender survives unchanged. **મોઢું**/*mouth*
is this track's clearest example of the nasalized **-ũ** neuter tell — the
same ending every **-વું** verb infinitive wears, now shown doing the same
job on a noun — while **નાક** is neuter without it, and **આંખ**'s medial
nasal (echoing વાંચવું's) is shown NOT to be a gender marker at all, so the
chapter keeps "nasal as sound" and "nasal as the neuter tell" explicitly
apart rather than letting a learner conflate them.

**Reach back at two cadences (HL09 §7).** Every lesson names atoms from the
one to three lessons immediately before it. Each chapter's payoff also
reaches back at least one chapter further: chapter 10's payoff rescues
chapter 6's `GU-SCRIPT-HEADLESS-CLUE` (never revisited since it was
introduced); chapter 11's rescues chapter 6's `GU-HISTORY-LEARNED-RESTORATION`;
chapter 12's rescues chapter 8's `GU-SCRIPT-ANUSVARA-MEDIAL` and chapter 7's
`GU-FORM-JAVU-JOVU-CONTRAST`, both previously orphaned. All three payoffs
close over their own chapter's atoms at **1.00 representativeness** (11/11,
9/9, 10/10) against the 0.5 floor.

The level gate's **reinforcement** criterion, previously vacuous at pre-A1
because chapters 1–5 are schema v1 and declare no atoms, now runs for real —
this tranche is the first pre-A1 content the track has ever had. It reports 4
non-etymology atoms at or below pre-A1 revisited fewer than twice
(`GU-HISTORY-TEA-CHA-ISOGLOSS`, `GU-LEX-BHAI`, `GU-LEX-DUDH`,
`GU-LEX-ROTLI`) after 8 etymology hooks are waived per the project owner's
standing decision. One atom that started at zero revisits,
`GU-GRAMMAR-GENDER-NOT-BY-ENDING` (rotli's payoff atom), was deliberately
rescued to two by genuine callbacks in `GU-C12-kaan` and `GU-C12-naak`,
both of which independently restate its point about gender not following
from a noun's ending. The remaining four are left for a future tranche
rather than padded with unearned reach-back — the standing instruction is to
report observed numbers, not re-pin them.

**Font check, done before authoring and again before commit.** A forced
XeLaTeX compile caught three real mistakes: a literal Chinese character
(茶) in Chapter 10's `ચા` prose, the Greek letter θ inside "Miθra," and
seven Devanagari-titled Wiktionary source links (Chapter 10 through 12) —
all fell through to Latin Modern Roman, which has none of the three, and all
were fixed (romanized to *chá*, *Mithra*, and English-titled Wiktionary
links) before this commit. The nasalized-vowel check (`ũ`, `ĩ`, `ã`,
combining U+0303) needed no new declarations: all three were already
handled by `book/preamble.tex`. The precise PIE notation this tranche uses
(`h₃ekʷ-`, `bʰréh₂tēr`, `swésōr`, `dʰewgʰ-`, `peh₃(i)-`, `nehas-`) — richer
than the plain-ASCII Pokorny style Chapters 7–9 used — compiles clean under
the corpus-wide font mapping now in place.

**Wiring**: `GU-PATH-013`–`GU-PATH-015` are three new path segments — one on
`SPINE-POLITE-REQUEST-REPAIR` (previously realized by zero segments), one on
`SPINE-EXCHANGE-NAMES`, one on `SPINE-CHECK-WELLBEING` — each with a matching
`GU-EXT-0{13,14,15}-LANGUAGE-SPECIFIC` extension. Both steps are required,
since `lessonSpineNodes` only walks `curriculum.path[].lessons`.

**Verification.** A forced XeLaTeX build of the 76-page book has zero
missing characters, zero new overfull/underfull boxes (the corpus's one
pre-existing underfull box, in an earlier chapter, predates this tranche),
and zero duplicate labels. All three new chapters generate as `voice`,
drivable end-to-end via the detachable `## The letters in this word`
section. Narration for all three chapters was read back and confirmed to
narrate correctly aloud — no markdown tables are used anywhere in this
tranche, so the narration generator's column-header quirk does not apply.
`npx vitest run tests/integration.test.ts tests/cli.test.ts` passes (19/19);
`check:modality`, `check:books` and `check:narration` all pass with no diff
beyond the new chapters. The six corpus-wide pinned-number tests
(`chapters`, `continuity`, `levels`, `modality-manifest`, `narration`,
`ramp`) shift with any authored content and are left failing, per standing
instruction — their numbers are reported here, not re-pinned.

## Chapters 8 and 9 — the eight verbs fifteen other tracks teach

Eight lessons on `SPINE-SAY-WHAT-I-DO`, split deliberately across **two**
chapters of four rather than one of eight. Eight lessons in one chapter would
have run to roughly twenty new atoms against `maxNewAtomsPerChapter: 12`, and
chapter 7 is already over that budget at sixteen; two chapters of ten each stay
inside it and each gets its own capability and its own payoff. Gujarati's
core-verb coverage moves **6/40 → 14/40 (35%)**.

### Chapter 8 — The Mind and the Page

Held together by what each root was doing *before* it named a mental act.

- **`GU-C08-vicharvun` — વિચારવું** *vichārvũ*, "to think" (`VERB-THINK`). The
  noun **વિચાર** *vichār*, "a thought," with a verb ending put on it: *vi-*
  "apart, thoroughly" on Sanskrit *car-* "to move, to range about," PIE
  \**kwel-* "to turn" (English *wheel*; Greek *kúklos* → *cycle*; Latin
  *colere* → *cultivate*, *colony*). The second block names the fact that
  *vichārvũ* was **built rather than inherited** — formed on the model of
  Sanskrit *vicarati* — and ties it to the learned restoration the learner
  already met in *traṇ*'s recovered *r*.
- **`GU-C08-samajvun` — સમજવું** *samajvũ*, "to understand" (`VERB-UNDERSTAND`)
  ← Sanskrit *sambudhyate*, *sam-* "fully" on **budh-** "to wake, to become
  aware" — the root of **buddha**, "the awakened one," and PIE \**bheudh-*,
  which English keeps in *bode*, *forebode* and *forbid*. An earlier draft also
  offered English **bid** as a cousin; that was cut. Modern *bid* fuses Old
  English *bēodan* (the genuine descendant) with an unrelated *biddan*, so it is
  not a clean cognate, and claiming it would have been exactly the
  not-really-a-cousin this track has twice refused. Second block: the Prakrit
  path *dhy* → *jjh* → *j* presented as **assimilate, then simplify** — the same
  two moves, in the same order, that took *dv* → *bb* → *b* in બે *be*.
- **`GU-C08-vanchvun` — વાંચવું** *vā̃chvũ*, "to read" (`VERB-READ`) ← Sanskrit
  *vācayati*, which is a **causative**: *vac-* is "to speak," so *vācayati* is
  "to **make** speak." Reading named as making a silent page talk. PIE
  \**wekw-* → Latin *vōx* → English *voice*, *vocal*, *vowel*, *vocation*,
  *advocate*. The script block is new work: every nasal dot the track had taught
  sat at the **end** of a word (*hũ*, *chhũ*, *-વું*), and this one sits in the
  **middle**, on the *ā* before ચ — matched to પાંચ *pā̃ch*, which the learner
  already counts with, and set beside the *javũ*/*jovũ* one-mark contrast.
- **`GU-C08-lakhvun` — લખવું** *lakhvũ*, "to write" (`VERB-WRITE`) ← *likhati*,
  where **likh-** meant "to scratch, to score" before it meant to write; its
  plainest Sanskrit relative is **રેખા** *rekhā*, "a line." A second **honest
  dead end** after *khāvũ*: the cousins beyond Indo-Aryan are disputed and the
  lesson claims none, offering instead the observation that Latin *scrībere*
  also meant *to scratch* and English *write* began as Germanic "to score."
  **Chapter payoff**, production: *hũ mārũ nām lakhũ chhũ*, "I write my name" —
  the first sentence in the track that puts a possessive, a noun and a new verb
  together.

### Chapter 9 — Taking, Asking, Helping, Liking

- **`GU-C09-levun` — લેવું** *levũ*, "to take" (`VERB-TAKE`) ← *labhate*, PIE
  \**lebh-*, whose meaning is worth the lesson on its own: not "seize" but
  "**work, harvest, gain, profit**." Secure cousins are Lithuanian *lõbis*
  "treasure" and Greek *láphura* "spoils of war"; Latin *labor* is frequently
  linked, which would make English *labour* a cousin, but the link is contested
  and at least one standard authority rejects it, so the lesson reports the
  dispute rather than settling it. That gives the track a three-setting dial it
  can now name: *jāṇvũ* **certain**, *levũ* and *jovũ* **disputed**, *khāvũ*
  **nothing**.
- **`GU-C09-puchhvun` — પૂછવું** *pūchhvũ*, "to ask" (`VERB-ASK`) ← *pṛcchati*,
  PIE \**prek-* — Latin *precārī* → *pray*, *prayer*, *deprecate*; *precārius*
  → **precarious**, "a thing you keep by asking"; German *fragen*, Persian
  *porsīdan*. English's own inherited cousin (Old English *frignan*) died out,
  so English asks with an unrelated word and prays with the related one. The
  sound block makes the *āpayati* → *āvvũ* rule earn its keep: a single Sanskrit
  *p* softened to *v* **between vowels**, and *pūchhvũ* keeps its hard પ because
  that *p* stands at the **front** of the word — as does the *p* of *pā̃ch*.
- **`GU-C09-madad-karvi` — મદદ કરવી** *madad karvī*, "to help" (`VERB-HELP`).
  **મદદ** is Arabic *madad* borrowed through Persian, on the root **m-d-d** "to
  stretch out, to extend" — the plainest sentence in that root being "extend
  your hand," so help is named as **reach**. It sits in the Perso-Arabic trade
  layer the track opened with *majā* in Chapter 3. The grammar is why this
  lesson is a phrase rather than a word: every Gujarati infinitive so far has
  ended in the **neuter** *-વું*, and this one does not. *Madad* is a
  **feminine** noun, the infinitive agrees with it, and the result is **કરવી**
  *karvī* against neuter *kām karvũ* — the three-gender system doing visible
  work rather than sitting in a chart, and a second sighting of gender choosing
  a form after બે *be*'s descent from the feminine/neuter *dvé*.
- **`GU-C09-gamvun` — ગમવું** *gamvũ*, "to like" (`VERB-LIKE-LOVE`). **મને
  ગુજરાતી ગમે છે** *mane gujarātī game chhe*, "to me Gujarati pleases," set
  directly against the Chapter-5 sentence *hũ gujarātī bolũ chhũ* so the learner
  hears themselves leave the subject slot. The etymology is the point rather
  than decoration: *gamvũ* ← Old Gujarati *gamaï* ← Prakrit *gammaï* "**is
  known**" ← Sanskrit *gamyate*, which is the **passive** of *gam-* "to go."
  The verb was born without a doer, which is why it has never taken one. Root
  PIE \**gwem-* — English **come** is the same word; Latin *venīre* gave
  *advent*, *venue*, *invent*. Two earlier atoms land here: *gammaï* "is known"
  is the meaning of **જાણવું** *jāṇvũ*, and the **y** inside *gamyate* did *not*
  become *j*, because that rule was for *y-* at the **front** of a word — a
  second condition-check after *pūchhvũ*'s. **Chapter payoff**, production.

  On the cross-linguistic note: the lesson names **Spanish** *me gusta* and
  **Tamil** *enakku piḍikkum* as unrelated languages reaching the same shape,
  and gives **no count** of how many tracks do so — a census in a lesson body
  goes stale on the next tranche. The claim that does the teaching work needs no
  census either: *gamvũ* is a **native** Indo-Aryan verb where Hindi and Urdu
  use the Persian loan *pasand*.

### Reinforcement

Both cadences from HL09 §7, deliberately. **Per lesson**, every one of the eight
names atoms from the immediately preceding one to three lessons in
`practises.knowledge`, including across the chapter-7/8 and chapter-8/9 seams.
**Per payoff**, both chapter payoffs reach back several chapters to rescue atoms
that had been introduced and never practised again — *khāvũ*'s missing-cousin
finding, *jovũ*'s unsettled origin, the *javũ*/*jovũ* contrast, the headless
script clue, both number histories, the intervocalic *p*, and both *jāṇvũ*
atoms.

Measured on the track: **12 orphans of 24 atoms → 3 of 44**. The three remaining
are the final lesson's own (`GU-LEX-GAMVU`, `GU-GRAMMAR-DATIVE-LIKING`,
`GU-ETYMON-GAM-GO`), which nothing follows yet. Corpus-wide the never-revisited
count falls 668 → 659 and the percentage 38 → 37, while atoms taught rise
1753 → 1773.

Every rescued atom is genuinely exercised in prose or drill; none was added to
`practises.knowledge` to make a number move. `reviews_of` is authored alongside,
but it names lesson ids and closes no window — the reinforcement claims above
all rest on `practises.knowledge`.

Both payoffs assess **10 of their chapter's 10** atoms, so representativeness is
**1.00** for each against the 0.5 floor.

### Book and fonts

Chapters 8 and 9 generate to `book/chapters/ch08-mind-and-page.tex` and
`ch09-doing-verbs.tex`. The nine-chapter build compiles under XeLaTeX at **54
pages with zero `Missing character` warnings**, verified by compiling rather
than assumed — `core/latex-warning-baseline.json` records Gujarati as `null`, so
CI cannot catch this.

Two font findings from that pass, both now recorded in the README. Source links
to **Arabic** مدد and **Sanskrit** गम्यते were emitting eight missing characters:
the book generator wraps only Gujarati-script runs in `\gu{}`, so Arabic and
Devanagari fell through to Latin Modern Roman, which has neither. Both link
titles are now romanized. And a throwaway probe confirmed that Latin Modern
Roman lacks `ʰ`, `ʷ`, `ḱ` and the subscript digits, so PIE roots are written
`*kwel-`, `*bheudh-`, `*wekw-`, `*prek-`, `*gwem-` in the plain style the rest
of the corpus already uses; `ā̃` (as in *pā̃ch*, *vā̃chvũ*) renders correctly with
no `newunicodechar` declaration. The long-standing Gujarati rule that
punctuation stays **outside** the `\gu{}` span still holds and is satisfied by
construction, since the generator groups only characters whose Unicode script is
Gujarati; a space inside a span is fine, so `\gu{મદદ કરવી}` is safe.

All eight lessons derive `coreModality: voice` with **zero** sight cues, so both
chapters stay drivable end to end. Their whole-lesson modality is `sight`
because each carries the canonical detachable `## The letters in this word`
block, which is the expected classification, not a defect.

## Chapter 7 — Six Verbs at the Core (the shared spine's verb node)

The Gujarati track's first lessons on `SPINE-SAY-WHAT-I-DO`, and its first
**canonical** verb concepts. Before this the track taught four verbs and every
one of them was namespaced (`GU-VERB-BOLVU`, `GU-VERB-RAHEVU`,
`GU-VERB-KARVU`, `GU-VERB-MALVU`), so none of them joined any other language.
Six lessons, six canonical tags: `VERB-BE`, `VERB-GO`, `VERB-COME`, `VERB-EAT`,
`VERB-SEE`, `VERB-KNOW`. Gujarati's core-verb coverage moves 0/40 → **6/40
(15%)**, and the four namespaced verbs stay as counted extras.

- **`GU-C07-hovun` — હોવું** *hovũ*, "to be." The chapter's organizing fact:
  **-વું *-vũ* is a neuter ending**, so every Gujarati verb is *named in the
  third gender* — the gender Hindi no longer has. Etymology: *hovũ* ←
  *bhavati* ← \**bheu-* (English *be, been, build, booth, bower*; *future*;
  *physics*). And the copula is named for what it is: **છે** *chhe* is **not**
  Hindi's *hai*. It comes through Old Gujarati *achhaï* from a Sanskrit verb of
  **dwelling/abiding**, while *hai* continues *asti* — the same ancient verb as
  English *is*. So English *is* and Hindi *hai* are relatives and Gujarati
  *chhe* is not one of them.
- **`GU-C07-javun` — જવું** *javũ*, "to go" ← Sanskrit *yāti*. One idea:
  **Prakrit turned word-initial *y-* into *j-***, given as a reusable decoding
  tool (*yoga* → *jog*, *yamunā* → *Jamnā*, *yava* → *jav*, *yuvan* → *jovān*).
  Grammar Lens re-lays the present as stem + person-ending + copula.
- **`GU-C07-aavvun` — આવવું** *āvvũ*, "to come." A first draft derived this
  from *ā-* + *yā-* ("go toward"), which would have been a tidy pairing with
  *javũ* and is **wrong**: the attested line is Old Gujarati *āvivaũ* ←
  Prakrit *āvei* ← Sanskrit ***āpayati***, on the root **āp-** "to reach." The
  correction pays better than the error did — *āp-* is Indo-European, Latin
  *apere* gives English *apt/aptitude/adapt/adept/inept*, and *co-* + *apere*
  gives *cōpula* → English **couple** and the grammarian's **copula**, which is
  precisely the word this track uses for *chhe*. The regular change behind it
  gets its own block: **a single Sanskrit *p* between vowels softened to *v***
  (*dīpa* → *divo*, *kūpa* → *kūvo*, and *dīpāvali* → **divāḷī**).
- **`GU-C07-khaavun` — ખાવું** *khāvũ*, "to eat" ← *khādati*, with the single
  intervocalic *d* worn away. Deliberately an **honest dead end**: the
  reconstructed root leaves nothing in English, and the lesson says so rather
  than reaching for a lookalike — "a cousin that is not really a cousin is
  worse than none at all." The anchor offered instead is the living Indo-Aryan
  set (*khānā*, *khāṇā*, *khāṇe*, *khāoyā*). Introduces the letter **ખ**.
- **`GU-C07-jovun` — જોવું** *jovũ*, "to see." One idea: it is **one vowel sign
  away** from *javũ* — bare **જ** against **જ** wearing **ો** — a pair that is
  as small and as total in the mouth as on the page. Its etymology is marked
  **probable, not proven**: Prakrit *joaï*, usually from *dyotate* "shines"
  (the shine → sight path English shows in *phenomenon*), possibly merged with
  an older verb of watching (Hindi *johnā*).
- **`GU-C07-jaanvun` — જાણવું** *jāṇvũ*, "to know" ← *jānāti*, root *jñā-*,
  PIE \**gnō-*. The chapter's widest cousin web — *know, knowledge, can,
  cunning, ken, uncouth; notice, note, notion, cognition, recognize, acquaint,
  noble, ignore; diagnosis, prognosis, agnostic* — set deliberately two lessons
  after the verb that had none. Introduces the retroflex **ણ** and names the
  Middle Indo-Aryan *n* → *ṇ* that produced it. This is the **chapter payoff**.

Track and infrastructure changes:

- `curriculum.json`: new path segment `GU-PATH-010` on `SPINE-SAY-WHAT-I-DO`
  (previously an empty node with 42 omissions); its omission ledger drops the
  six now-realized concepts and stands at 36.
- `chapters.json`: chapter 7 capability entry, payoff `GU-C07-jaanvun`
  (production). Payoff representativeness 9/16 introduced atoms (0.56), above
  the 0.5 floor; chapter 7 raises no HL05 finding. The chapter introduces 16
  knowledge atoms, above the (currently unenforced) `maxNewAtomsPerChapter` of
  12 — a six-lesson verb chapter is genuinely denser than the corpus median,
  and the number is not padded down to meet a threshold.
- `core/book-generation.json` + `book/book.tex`: chapter 7 generated from the
  same lesson AST as everything else (`ch07-core-verbs.tex`). XeLaTeX build is
  38 pages with **zero** `Missing character` and zero undefined references.
  Latin punctuation stays outside every `\gu{}` span, as this font requires.
- `data/scripts/gujarati.json`: added **ખ** *kha* and **ણ** *ṇa*, the two
  letters this chapter needs, so no headword raises an uncovered-glyph warning.
- Modality: all six lessons derive `voice`. Chapter 7's drivable prefix is
  6 of 6 — the first Gujarati chapter after chapter 6 that a commuter can do
  end to end. No tables, no sight cues, letters taught in prose.
- Durations (computed, sub-300s contract): hovun 281s, javun 267s, aavvun 281s,
  khaavun 260s, jovun 270s, jaanvun 253s.

## Chapter capability ledger — 2026-08-06

- Added `chapters.json`, the HL05 chapter capability ledger, covering Chapter 6:
  the reader can count *ek, be, traṇ, chār, pā̃ch* in headless Gujarati script
  and explain the track's two odd numerals.
- Made `GU-C06-number-histories` the chapter payoff — the chapter's last
  schema-v2 lesson by sequence (350), and the one that pays the chapter's
  promise: **બે** from feminine/neuter *dvé* through *dv → bb → b*, and **ત્રણ**'s
  *r* as a learned restoration after Prakrit *tiṇṇi* had already lost it.
- Recorded `SPINE-COUNT-ONE-TO-FIVE` as the chapter's spine node, matching
  `GU-PATH-009` in `curriculum.json`.
- Omitted Chapters 1–5 rather than stubbing them: all 31 of their lessons are
  schema v1 and declare no `practises.knowledge`, so no payoff there could name
  atoms a lesson actually exercises. Their absence is the debt the HL05 gap
  report exists to measure.
- Measured payoff representativeness for Chapter 6 at 7/8 introduced atoms
  (0.88). The one atom outside the payoff is `GU-SCRIPT-HEADLESS-CLUE`, which
  the histories lesson does not re-exercise; it was not padded in.

## Warning-clean six-chapter book — 2026-08-03

- Replaced the five duplicate recap labels with canonical lesson ids and moved
  Latin punctuation outside the Gujarati-only font command.
- Preserved readable Gujarati in PDF bookmarks while removing font-only
  presentation commands from Hyperref's strings.
- Added natural page bottoms, explicit static-font style mappings, and a
  breakable copula recap; the forced 27-page build is now warning-free.

## Canonical Chapter 6 publication — 2026-08-03

- Migrated both number lessons to schema v2 with the shared
  `SPINE-COUNT-ONE-TO-FIVE` can-do node, explicit sub-five-minute budgets, and
  block-level knowledge closure.
- Generated the downloadable Chapter 6 from the same ordered lesson AST and
  source hash that Language Ladder loads, rather than maintaining another copy.
- Preserved Gujarati script inline with the book's vendored font and used
  romanized section short titles for stable PDF bookmarks.

## Sub-five-minute remediation — 2026-08-02

- Corrected eight declared five-minute estimates whose computed durations were
  already between 110 and 184 seconds.
- Split the genuinely long numbers lesson into a 174-second counting lesson and
  a prerequisite-ordered 253-second etymology lesson.
- Preserved the complete *dvé → be* assimilation history, the comparison across
  Hindi, Marathi, and Bengali, and the restored *r* in *traṇ*. The shared report
  now measures zero Gujarati duration violations.
- Updated the roadmap and session map to expose both Chapter 6 lesson boundaries.
  Chapter 6's missing one-source book publication remains explicit in the shared
  backlog.

## Chapter 6 — Numbers 1–5, and two different inheritances

- **Chapter 6 authored** (`GU-C06-numbers-1-5`): *ek, be, traṇ, chār, pā̃ch* —
  romanizing the anusvāra with the **tilde**, as every other lesson in this track
  does (*hũ*, *chhũ*, *mārũ*), rather than the plain *n* a first draft used.
- Three of the five match every neighbour. The chapter is about the **two that
  don't**, and neither is an accident:
  - **બે *be*** — where Hindi, Marathi and Bengali all say something with a *d*
    (*do*, *don*, *dui*), Gujarati says *be*. Sanskrit had **different forms for
    different genders**, and Gujarati continues the **feminine/neuter *dvé***
    while **Hindi and Marathi** continue the masculine *dváu*. (Bengali is
    deliberately **not** lumped in with them: its *dui* continues the disyllabic
    Prakrit *duve*, which is where its second vowel comes from — an earlier draft
    had Gujarati's surfaces claiming Bengali was on the *dváu* side, contradicting
    the Bengali chapter shipping in the same commit.) A **different inheritance**
    from the same paradigm — explicitly not a corruption. The cluster's fate is
    stated properly too: the *d* took on the **labial place of articulation** of
    the following *v* (*dv* → *bb* → *b*) — a dental stop pulled to the lips —
    rather than "softening away" as a first draft had it.
  - **ત્રણ *traṇ*** — the interesting correction. Both *traṇ* and Hindi's *tīn*
    carry the **ṇ**, which betrays that both descend from the **neuter *trī́ṇi***
    via Prakrit *tiṇṇi* — where the *r* had **already been lost**, its weight
    transferred into the doubled *ṇṇ*. So Gujarati's *tr-* is generally treated
    as **restored** under the influence of Sanskrit (which stayed a living
    literary language and kept reaching back into its descendants), not carried
    through unbroken. The lesson's point becomes sharper for it: *traṇ* **looks
    older than *tīn* and in a sense isn't** — it's closer to the original because
    someone put the *r* back.
- Names the script fact that's visible on the page: **Gujarati is Devanagari
  without the top line** — same letters, same system, the shirorekhā simply not
  drawn. Anyone who did the Hindi writing track can feel the relationship at once.

## Chapters 1–5 — new Gujarati track (Gujarati script taught inline)

New Gujarati track on the HL00 framework — Indo-Aryan, written in the Gujarati
script (vendored Noto Sans Gujarati font, `data/scripts/gujarati.json`). One
word/phrase per lesson, slug ids, atom-first assembly, every atom traced to its
root, a publishable LaTeX book. No reading course: the script — the "headless"
Devanagari-without-the-top-line — is taught *inside* each word lesson.

- **Chapter 1 — Greetings** (`lessons/GU-C01-*`): namaste, ābhār (Sanskrit
  *bhṛ* "to bear," cousin of English *bear*, Portuguese *obrigado*), hā/nā,
  sārũ (introduces the **three genders** *sāro/sārī/sārũ*), āvjo ("come again"),
  practice. Foregrounds the two Gujarati distinctives from the first page: the
  **missing top line** and the **three genders**.
- **Chapter 2 — Introducing Yourself** (`lessons/GU-C02-*`): nām (PIE
  *h₃nómn̥*, English *name*), mārũ (gender agreement again), **chhe** (Gujarati's
  own copula, not Hindi *hai*), "mārũ nām … chhe", tũ/tame (courtesy-by-plural),
  shũ ("what," the odd *sh-* cousin in a *k-* family), "tamārũ nām shũ chhe?",
  ānand ("joy"), practice.
- **Chapter 3 — How Are You** (`lessons/GU-C03-*`): kem (the *k-* questions,
  PIE *kʷo-*), "tame kem chho?", hũ (*aham* → Latin *ego*, English *I*), **majā**
  (Persian *maza* — the Perso-Arabic trade layer), vāndho nahī, practice.
- **Chapter 4 — Farewells** (`lessons/GU-C04-*`): pāchhā, maḷīshũ (the future
  is an ending; the retroflex **ḷ** ળ Gujarati keeps), "pāchhā maḷīshũ", kāle
  (*kāl* = both "tomorrow" and "yesterday"), practice.
- **Chapter 5 — The First Verbs** (`lessons/GU-C05-*`): bolvũ (the *-vũ*
  infinitive; stem + person + copula), "hũ gujarātī bolũ chhũ" (*gujarātī* ←
  the **Gurjar** people), rahevũ (postposition *-mā*), kām karvũ (Sanskrit *kṛ*
  — root of *namaskār*, *karma*), practice.

Infrastructure: vendored `_fonts/NotoSansGujarati-Static.ttf` (shaping verified);
`data/scripts/gujarati.json` (29 letters, 10 marks, abugida) per the HL01 schema;
`book/preamble.tex` with the `\gu{}` command and IAST `newunicodechar` maps.
Book compiles clean with XeLaTeX (0 missing characters, 0 undefined references)
and was rasterized and visually QA'd. Note for this script: Noto Sans Gujarati
carries no Latin punctuation glyphs, so all `.?!-` are kept **outside** the
`\gu{}` spans (they tofu inside).
