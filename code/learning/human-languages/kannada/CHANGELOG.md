# Changelog

## Unreleased — chapters 67–73: the joining column, closed, and eight letters

Twenty-seven headwords and eight script lessons, in seven chapters of five. Not
one was chosen by topic. Every one came off the uncovered list in
`core/exam-inventory-kannada-a1.json`, largest cluster first, because an
uncovered exam point NAMES the missing word and a vocabulary count does not.

    kannada A1 exam coverage      167/258 (65%) -> 193/258 (75%)
    joining column                  2/11        -> 11/11   (complete)
    negation                        4/5         -> 5/5     (complete)
    determiners                     1/2         -> 2/2     (complete)
    questions                       5/9         -> 8/9
    communicative functions        32/51        -> 42/51
    corpus exam-point backlog     786 -> 851    (this branch also lands the
                                                 inventory; see below)
    kannada lessons               268 -> 303
    kannada atoms taught          362 -> 412
    script characters TAUGHT       42 -> 50     (of 69 used in headwords)
    reinforcement misses          966 -> 895    (improved)
    atoms never revisited          20 -> 15     (improved)
    forward references             13 -> 13     (held)
    duration violations             0 -> 0      (held)
    never-taught glyphs             0 -> 0      (held)
    load-bearing closure viol.     10 -> 10     (held)
    distinct Kannada characters    69 -> 69     (held — no new glyph spent)

Every figure re-MEASURED against the merged tree. The denominator did not move:
no point was added, removed or reworded, and `partial` stayed at 0, so every
probe still names atoms that exist.

### The finding this answers, and what it cost to answer

The inventory measured the joining column at 2 of 11 and neither covered point
was a conjunction. `mattu` (and), `athava` (or), `aadare` (but), `eekendare`
(because) and the quotative `anta`/`endu` occurred **zero** times in 268 lessons
and zero times in the generated book — verified again here in both scripts, in
romanization, in markdown and in the compiled `.tex`.

Twenty-seven headwords closed twenty-six points, and nine of them are this
column. That ratio is a consequence of reading the list before designing the
chapters rather than after: `athava` closes "or" AND unblocks expressing a
preference; `alla` closes the negation pair AND gives the only way Kannada has
to disagree AND is half of the confirmation tag; the `-aa` question particle,
which no lesson had ever NAMED, closes asking whether somebody knows and asking
what they like at once.

### Order by dependency, not by topic

`anta` is taught before `eekendare`, because `eekendare` is literally *eeke*
("why") plus *endare*, the written quotative wearing the same `-dare` ending
that makes `aadare` out of the verb `aagu`. The learner meets the pieces, then
the word. Likewise `yaavaaga` before the `-aaga` when-clause, `ondu` before
`innondu`, `heelu` before "say it once more", and the quotative before the
opinion frame that needs it.

Kannada's real clause-chaining machinery — the `-i` participle of *hoogi
baruttene* — was already properly taught, and chapter 73 names it beside `-aaga`
and `-alu` as three shapes of one habit rather than three unrelated endings.

### Why NOT chapter 64

Chapter 64 is named `JOIN` and its five words connect turns, not clauses. It was
left alone. Its five words are a coherent set doing a real discourse job, the
chapter's `canDo` promises exactly that job, and cramming eleven clause-joiners
in beside them would have broken both the promise and the one-new-item-per-lesson
rate. The new material is in new chapters, and `KA-A1-CJ-10`'s note now records
that decision rather than leaving it to be rediscovered.

### The script: eight characters, one of them with a cited stroke order

The inventory measured 42 characters taught against 69 used in headwords. The
eight added are the eight most-used untaught ones — `ma` (36 headwords), `la`,
`va`, the vowel sign `ee`, `tta`, `ja`, `a` and the vowel sign `uu` — so a
learner who finishes the ladder can now read *maatanaaDu*, the verb chapter 5 is
built on.

Seven are recognition lessons in the ladder's established shape, which says in
as many words that this book does not yet know where the pen starts. **`a` is
different: its stroke order is SOURCED**, from Gopala Krishna A's 35-frame
Wikimedia Commons animation already cited in `data/scripts/kannada.json`, and
the lesson prints the four movements, the pen-lift count and the citation. It
also says why it is different from the seven before it.

Nineteen characters remain untaught and were left untaught. Six are independent
vowels whose ductus IS sourced and simply has not been spent; thirteen have no
source anywhere in this project and none was invented for them. `HL-C311`
records both lists and the method.

### Reinforcement went DOWN, and the parts are separated

A naive append measured 1040 misses against a prior 966. Decomposed:

    on atoms this tranche introduces        48
    on OLD atoms, window judgeable before  934   (966 before; 32 repaired)
    on OLD atoms, window NEWLY judgeable    58   (pre-existing debt exposed
                                                  by the extra length)

Three changes brought the total to **895**, below the starting point. Each
lesson retrieves at the R2 and R3 intervals as well as R1, arranged from the
ATOM's side — for every lesson here, the first WORD lesson standing at the right
distance is given its atoms to ask for. Each lesson also retrieves up to four
OLD words whose R4 window it can fill, naming their lessons as prerequisites,
because Kannada's prerequisite chain reaches only 113 of its 268 existing
lessons and the validator was right to refuse them otherwise. Only LEXICAL atoms
were taken for the distant retrieval: "say it once more" retrieves a word and
does not retrieve a claim about its etymology.

Final split: **3** misses on this tranche's own atoms, **868** pre-existing,
**24** newly exposed. Every declaration is backed by a `[YOU RECALL: …]` line
that asks for the words.

### Three ratchets held rather than reseated

Forward references stayed at 13. Making `bēku` a headword would have turned two
earlier passing mentions into forward references, so the lesson teaches the
fuller utterance `ನನಗೆ ಚಹಾ ಬೇಕು` — which is what a learner actually says, and
what `KA-A1-F-21` asks for. Chapter payoffs stayed at 4 findings: each new
chapter's payoff gathers everything the chapter taught up to that point, which
is what a payoff is for. Durations stayed at 0 violations, computed rather than
declared — the longest new lesson lands at 271s against the 300s ceiling, and
the opinion lesson was cut from 296s by removing prose rather than by editing
its declared number.

### Spent no new glyph, and checked the examples

All 27 headwords and every worked example are spelled from characters already in
the corpus. The distinct-character count held at 69.

## Unreleased — the A1 exam inventory: 167 of 258, and no conjunction anywhere

`core/exam-inventory-kannada-a1.json` enumerates **258** A1 points and measures
the corpus at **167 covered, 65%** — the second-highest of the nine inventories
now written, behind nothing and level with Telugu. Kannada becomes the ninth
track that can be *measured* at all; fifteen still cannot.

It is not only a gate. `npm run plan` can already say a track needs vocabulary;
it cannot say WHICH words. The 91 `probe: null` points name them, and are
written as a work queue for that reason.

### Derived from the Spanish A1 set, and accounted for in both directions

The point set is derived structurally from `core/exam-inventory-es-a1.json`, the
DELE/PCIC-sourced Spanish A1 inventory. **Spanish is a proxy for LEVEL, not for
grammar**: each Spanish point was read as "what must an A1 learner handle here",
and the Kannada exponent carrying the same load was then chosen. Where Kannada
carries it with machinery Spanish has no name for — a case suffix, a dative
subject, a zero copula — the point is restated around the Kannada machinery and
records its origin.

All **273** Spanish points are attributed: 271 derive into some Kannada point,
and exactly 2 are dropped with a reason (the *al/del* contractions, and the
exclamative *qué*). Four Kannada points have no Spanish question behind them at
all and are flagged `kannadaSpecific` — the human/non-human noun division, the
one-suffix-per-job property of an agglutinating language, the p→h sound law, and
the spoken/literary divide.

An earlier draft had **fifteen** Spanish points both derived *and* dropped,
because restating a point around Kannada machinery feels like not transferring
it. Restating is deriving. `notTransferred` now holds only points that produce
no Kannada point at all, and a test enforces the disjointness.

### The joining column: 2 of 11, and neither is a conjunction

`mattu` (and), `athava` (or), `aadare` (but), `eekendare` (because) and the
quotative `anta`/`endu` occur **zero** times in 268 lessons and zero times in the
generated book. The two covered points are the `-i` participle, which is
Kannada's real clause-chaining machinery and is properly taught in chapter 4,
and chapter 64 — a chapter *named* `JOIN` whose five words all connect turns
rather than clauses. Three function points and one taught verb are idle for want
of words that are each one lesson long.

### The script column is the opposite of Tamil's

Measured, not assumed: **42 characters taught against 69 used in headwords, 61%
closure**. Tamil's inventory reported 52 of 52 taught; an inventory that carried
that expectation over would have written false notes here. `ma` appears in 40
headwords and is never taught.

### Verified

Every atom named in every probe was checked to exist **against the merged tree** —
37 of them did not exist until Kannada chapters 1, 2 and 4 were retired into the
generator earlier the same day, so validating against the working branch would
have been validating against a tree nobody else had. One invented id
(`KA-ETYMON-C38-HRUDAYA-01`, which is `KA-LEX-…`) was caught by that check and
fixed.

The corpus-wide exam-point backlog moves **792 → 883 across 9 inventories**, and
the unmeasurable remainder falls **16 → 15 tracks**. Re-measured with `npm run
plan` against the merged tree, never composed: this branch read 793 before the
chapter retirement landed, and the retirement moved the number on its own.

human-language-data 124 test files / 1745 tests (7 new), all eleven `check:*`
gates, language-ladder 39 files / 442 tests.

## Unreleased — Kannada owns no hand-written chapters

Chapters 1, 2 and 4 are generated. With chapters 3 and 5 already retired,
`handwritten_parity.py --check kannada` now answers **"already retired, nothing
handwritten remains"** and exits 0. The track is closed.

Nineteen more `KA-C0[124]-*` lessons moved from schema v1 to v2, gaining 37
knowledge atoms. The three `practice` lessons sat in no curriculum path segment,
which v2 forbids for a lesson declaring a spine node; each is appended to the
segment that already holds its prerequisites (`KA-PATH-003`, `-004`, `-009`)
with a matching consolidation extension, so no segment had to be inserted and no
path shard had to be renumbered.

### The pipeline learned the block type the corpus had already authored

Two lessons — `KA-C01-dhanyavada` and `KA-C01-illa` — carried a heading
`## Across the family — the same idea, five ways` that **no block type matched**.
It classified as `unknown`, which schema v2 rejects, so those two lessons could
never have been generated. The books have drawn that comparison in their own
violet `cognates` box since the beginning; the pipeline simply had no name for
it. `parse.ts` now classifies it, exactly as it already does for
"The letters in this word", and `book.ts` renders it.

It renders as an **inline** `tcolorbox` rather than `\begin{cognates}`: only the
four Dravidian preambles define that environment, so emitting the named one
would compile here and break the moment any other track used the heading. The
colours travel in the emitted LaTeX instead, so the page is unchanged.

### Prose the parity script could not see

`handwritten_parity.py` reported 6 blocks at risk. Six were at risk — but not
the six it named. It compares COUNTS per environment, so chapter 1's two
`culture` blocks matched its lessons' two "Why it's said this way" sections and
the gate reported nothing, while in fact **one .tex block had no lesson and one
lesson block was not in the .tex**: equal counts, different sets.

Matching blocks by CONTENT instead (Jaccard over tokens, which survives the
rewrapping the prose underwent on its way into LaTeX) found the real list:

| reported by parity | actually orphaned |
|---|---|
| ch1 `sounds` ×1 | **false** — `KA-C01-namaskara`'s script block already covers it, *ottakṣara* included, and that term is also in `KA-C01-practice` |
| ch1 `cognates` ×3 | **1** — two were already authored in the lessons; only the *namaskāra* "Hello" table was missing |
| ch1 — *(not reported)* | **1 `culture`** — the "go and come back" farewell |
| ch2 `cognates` ×1 | **1** — the *hesaru* "Name" table |
| ch2 — *(not reported)* | **1 `cousinweb` clause** — the \*nān pronoun stem behind Tamil *nāṉ* / *eṉ* |
| ch4 `cognates` ×1 | **1** — the "go and come back" table |

Three cognate tables were carried across from the `.tex`, plus the *nān stem
clause into `KA-C02-nanna`. The chapter-1 `culture` block was **not** copied
anywhere: `KA-C04-hoogi-baruttene`'s own etymology block already teaches all of
it. It was added, the page was compiled and looked at, and the same idea printed
three times on one spread — so it came back out. Measured at 64% token overlap
with zero unique facts.

### Four forward references resolved, deliberately

Generating chapters 1 and 2 drops four Kannada words from them — ಹೋಗಿ,
ಬರುತ್ತೇನೆ, ಬನ್ನಿ and ನಾನು. That is the point. `KA-C01-practice` already says so
in its own words: the departure exchange *"belongs in the later chapter where
you learn those actions one at a time."* The hand-written `.tex` taught them
three chapters early anyway. All four survive in the generated book (11, 13, 1
and 25 occurrences) and each now first appears in the chapter that teaches it.
`measureContinuity` reports Kannada's forward references at **13**, unchanged.

### Two more defects only the page showed

Ten further lessons carried the duplicated `Sounds you'll need` / `The letters in
this word` pair described in the previous entry; all ten are resolved the same
way, each proven a strict subset first. Three lessons — `KA-C02-enu`,
`KA-C04-hoogu`, `KA-C04-naale-sigona` — were not NFC-normalized and now are,
which completes the seven the corpus had.

`KA-C01-practice`'s recap table was **four columns with an unlabelled second
column** holding the romanization — the exact shape `chapter-policy.json` calls
unspeakable, and the narrator refused it. The romanization now sits beside its
Kannada in the `Read` cell, making three labelled columns, so the narrator reads
the table instead of refusing it. The corpus refusal pin moves 54 → 53.

### One bug fixed outside the track

`hindi/lessons/HI-C03-thik.md` had raw LaTeX leaked into its Markdown —
`long *\=\i*` where it meant `long *ī*`. Hindi chapter 3 is already generated,
so the Hindi book has been **printing `long =\i` to readers**. Found by grepping
the corpus for the same leak after `KA-C02-niinu-niivu` turned out to have one;
those two files were the only cases in 22 tracks.

### All five payoffs were below the representativeness floor

`chapter-policy.json` sets `payoffRepresentativeness: 0.5` — a chapter's payoff
should assess at least half of what the chapter introduced. Every one of the five
newly generated chapters failed it, because each payoff had been anchored to a
hand-picked highlight from the practice lesson rather than to what that lesson
actually does:

| chapter | was | now |
|---|---|---|
| 1 | 5/14 = 0.36 | 8/14 = **0.57** |
| 2 | 3/17 = 0.18 | 11/17 = **0.65** |
| 3 | 2/15 = 0.13 | 11/15 = **0.73** |
| 4 | 2/12 = 0.17 | 9/12 = **0.75** |
| 5 | 3/9 = 0.33 | 8/9 = **0.89** |

The payoff *is* the practice lesson, so `assesses` is now everything that lesson
practises which this chapter introduced — no widening of what is taught, just an
honest reading of what the closing checkpoint already covers. The corpus-wide
count of payoffs below the floor moves **91 → 86**, measured before and after.

This was invisible while the chapters were v1: a schema-v1 lesson declares no
atoms, so the denominator was empty and the floor could not be computed at all.

Counted from the directory of the merged tree, never derived: `handwritten.d`
holds **31** entries on `origin/main` and **28** on this branch. **Kannada's
count is 0.** The corpus-wide figure has moved 38 → 31 while this work was in
flight, as Arabic, Urdu and several French and German chapters were retired
alongside it, which is exactly why it is re-counted rather than adjusted.

The corpus-wide narrator refusal pin moves to **51**, measured against the
merged tree rather than derived: main had already moved 54 → 52 for French
chapter 7 while this branch held 53.

Verified: human-language-data 124 test files / 1730 tests, all eleven `check:*`
gates, language-ladder 39 files / 442 tests, and the whole Kannada book compiled
under XeLaTeX — 433 pages, **zero missing characters**, zero overfull and zero
underfull boxes. The pages carrying every rescued block were rasterised and read.

## Unreleased — chapters 3 and 5 are generated, and stop being hand-written LaTeX

`ch03-responding.tex` joins `ch05-first-verbs.tex` in the generator. Both were
`handwritten` ledger entries; Kannada owned five and now owns **three** —
chapters 1, 2 and 4, each of which still holds `cognates` prose no lesson has.

Eleven `KA-C0[35]-*` lessons moved from schema v1 to v2 (v1 declares no
knowledge atoms, and `book.ts` refuses to generate from a v1 lesson), gaining 19
atoms between them. `KA-C03-practice` sat in no curriculum path segment, which
v2 forbids for a lesson declaring a spine node; it now sits in a new
`KA-PATH-008-RECAP` under `SPINE-CHECK-WELLBEING`, placed after
`KA-PATH-008` so it still follows `KA-C03-paravaagilla`, with a matching
`KA-EXT-007-CONSOLIDATION`.

### Two defects that only the rendered page showed

**The same syllable breakdown printed twice, under the same title.** Fourteen
chapter-1-to-5 lessons carry both a `## The letters in this word` block and a
`## Sounds you'll need` block. The hand-written LaTeX printed only one of them.
Generated, both print — and because Kannada's preamble titles the `sounds` box
*"The letters in this word"*, the reader met that heading twice on one page with
near-identical text under each. No already-generated Kannada chapter carries
both blocks (0 of 229), so the duplicate was removed from the four lessons in
these two chapters — but only after proving mechanically that the removed block
was a strict subset: every non-ASCII character and every token of it already
appears in the block that stays.

**Four lessons were not NFC-normalized.** The proof above failed at first on
`hē` not equalling `hē` — one precomposed U+0113, one `e` plus a combining
macron U+0304. Seven of Kannada's 268 lessons carry decomposed romanization and
all seven are in the hand-written chapters. The four in chapters 3 and 5 are now
NFC; the remaining three (chapters 2 and 4) go with their own chapters.

### Nothing was dropped, and that was measured rather than asserted

`handwritten_parity.py` reported both chapters clean. That agreed with the
artifacts, but its block-gap number is not trustworthy alone in either
direction — it counts LaTeX environments, so a word taught in a paragraph inside
a surviving box costs zero blocks while costing a whole lesson. So both chapters
were sized a second way, by counting the words the `.tex` teaches against the
lessons that own them, and each flip was gated on token loss against the
pre-migration tree:

| chapter | Kannada tokens | lost | tokens owned by no lesson |
|---|---|---|---|
| 3 | 28 → 33 | **0** | 0 |
| 5 | 22 → 34 | **0** | 0 |

`cousinweb` goes 4 → 6 and 3 → 5, `grammarlens` 2 → 3 and 4 → 4, `sounds` 3 → 3
and 1 → 1. The chapters grow because the generator publishes the warm-up and
wrap-up-recall blocks the hand-written versions dropped.

One genuine discrepancy surfaced: the hand-written book romanised Tamil
எப்படி as *eppaṭi*, the lessons as *eppaḍi*. The book's form is restored, in
three places — a deliberate three-character change the census reports exactly.

### A ramp number moved, and it is coverage arriving rather than a regression

Chapter 3 became the track's first `atomChapterSpikes` entry (0 → 1), and
`atomsNeverRevisited` went 9 → 11. Nothing got steeper: the chapter was schema
v1, which declares no atoms at all, so it was invisible to every ramp
measurement until now. Fifteen atoms across eight lessons is **1.9 per lesson**,
under the corpus mean of 2.31, and the chapter introduces five new words across
six content lessons — one new word per lesson. It trips
`maxNewAtomsPerChapter: 12` because it is long, not because any step is large,
and `maxNewAtomsPerLesson: 3` — the budget that defines gentle — is met exactly.
The atoms were deliberately NOT merged to get under the number; the honest
remedy is splitting a chapter that already spans two spine nodes, which is
recorded in `BACKLOG.d` rather than done inside a retirement PR.

Counted against the merged tree after merging it, never derived: `handwritten.d`
holds **35** entries on `origin/main` (sibling PRs retired a French and a German
chapter while this was in flight) and **33** on this branch.

Verified: human-language-data 124 test files / 1730 tests, all eleven `check:*`
gates, language-ladder 39 files / 442 tests, and the whole Kannada book compiled
under XeLaTeX — 421 pages, zero overfull and zero underfull boxes — with the
changed pages read on the page rather than in the source.

## Unreleased — counting one word at a time, and a script ladder that arrives before the words it reads

Chapter 7 taught all ten numbers in **two** lessons, each opening on a
five-row, four-column reveal table. It is now **ten** lessons — one number
word each, one new knowledge atom each — and the ten Kannada digits ೦–೯ ride
behind them as ten one-glyph script lessons spread across chapters 9–18.

Separately, the script ladder was resequenced. It used to start at chapter 6,
which meant the first thirty-three lessons printed Kannada that no lesson had
introduced. Every letter lesson now sits at the last position that still
precedes its first consumer **and** still follows the word it cites.

Measured (`measureScriptClosure`, `measureContinuity`, `measureRamp`):

- closure violations **30 → 10**
- glyphs shown but never taught **10 → 0**
- headwords with no romanization **14 → 0**
- distinct headwords **204 → 212**
- glyph spikes **6 → 4**; chapter atom spikes **0 → 0**; forward references **13 → 13**

### Chapter 7 — one headword per lesson

`KA-C07-numbers-1-5` and `KA-C07-numbers-6-10` are replaced by
`KA-C07-ondu`, `-eradu`, `-muuru`, `-naalku`, `-aidu`, `-aaru`, `-eelu`,
`-entu`, `-ombattu`, `-hattu`. Each teaches one word and one atom, and each
carries exactly one of the chapter's ideas rather than bundling five:

  ondu     Telugu breaks rank (okaṭi)        aaru     Tamil's rare ṟ flattens
  eradu    Tamil ṇṭ softens to ḍ             eelu     Tamil ḻ hardens to ḷ
  muuru    the long ū the family kept        entu     where the family parts
  naalku   the virama holds ಲ್ಕ together      ombattu  the anusvara's third face
  aidu     the softening, a second time      hattu    the p → h shift

`KA-LEX-C07-NUMBERS-6-10-01`, `KA-ETYMON-C07-NUMBERS-6-10-02` and
`KA-ETYMON-C07-NUMBERS-6-10-03` are kept and re-homed on the lessons that
still teach them, so chapters 33 and 34 need no edit to their `requires`.

The chapter's two wide tables are gone with the lessons that held them, which
is why the corpus-wide narration refusal ceiling drops 60 → 58.

### The digits, spread rather than blocked

Ten digits in one chapter would be an alphabet block, and chapter 7 has an
atom budget of twelve. So `KA-S140`–`KA-S149` teach ೧ ೨ ೩ ೪ ೫ ೬ ೭ ೮ ೯ ೦ one
per chapter across chapters 9–18, each riding a number word the reader has
already been saying.

The closure module credits a script lesson with **every** glyph in its body, so
each digit lesson was measured for what it actually credits. Seven credit only
their own digit. Three credit one letter as well — `KA-S140` (೧) is the first
teaching lesson to print ಒ, `KA-S142` (೩) the first to print ೂ, `KA-S144` (೫)
the first to print ಐ — because those three vowels appear in no word before
chapter 7 and the letter ladder has no lesson for them. That is real debt: the
digit lessons are carrying three letters they do not teach. It is recorded in
the backlog rather than hidden by removing the words, which would have pushed
`neverTaughtGlyphs` back up to 3.

### The ladder, resequenced

The two letter chains kept their relative order except where a letter's word
did not yet exist. `KA-S110-letter-ba` moved to chapter 4, because ಬ appears
in no word before ಹೋಗಿ ಬರುತ್ತೇನೆ; `KA-S114-vowel-sign-o` moved to chapter 6 for
the same reason with ಗೊತ್ತು. `KA-S111-vowel-sign-e` becomes the chain head.
Ten letter lessons now land in chapters 1–5, whose book chapters are
handwritten, so each is embedded there with a `canonical-insertion` marker and
declared in `core/book-generation.d/handwritten.d/kannada-000N.json`.

Example lists were trimmed wherever a letter lesson said "you already say
these" about a word the reader had not met — that claim was false at the new
positions, and fixing it is what keeps forward references at their 13 baseline
rather than 42.

### What did NOT move, and why

**Pre-A1 vocabulary is unchanged at 152/300.** Chapter 7 is staged `A1` in
`KA-EXT-011-LANGUAGE-SPECIFIC`, matching Tamil, Telugu, Malayalam and Hindi,
whose counting nodes are all `A1` too. Ten new headwords therefore land at A1,
not pre-A1. Restaging the chapter would have moved the number by relabelling
rather than by teaching, and would have broken parity with four sibling
tracks, so it was not done. See the backlog entry.

**Chapter payoff representativeness got worse**, from 3 payoff surprises to 9:
chapters 1–5 are schema-v1 with empty `assesses`, and they now contain script
lessons whose atoms no payoff claims. That is recorded debt, not a regression
in the teaching.

### The ten violations that remain

`KA-C01-illa`, `KA-C01-practice`, `KA-C02-practice`, `KA-C02-santosha`,
`KA-C03-cennaagi`, `KA-C04-hoogu`, `KA-C04-naale-sigona`, `KA-C05-iru`,
`KA-C05-kelasa-maadu`, `KA-C09-kshamisi`. Their blocking glyphs, each traced
to the lesson that first teaches it:

- **ಅ** (`KA-C02-practice`, `KA-C05-iru`, `KA-C05-kelasa-maadu`) and **ಭ**
  (`KA-C02-santosha`, `KA-C09-kshamisi`) exist only inside the 38-glyph grid
  lessons `KA-S124-letter-pa` (ch. 15) and `KA-S122-letter-ca` (ch. 13).
  Moving a 38-glyph grid to chapter 2 would be an alphabet block, so the real
  fix is to split the grids into single-letter segments.
- **ಓ** (`KA-C04-naale-sigona`) waits for `KA-S126-letter-u` at chapter 16.
- **ಬ** (`KA-C04-hoogu`) cannot be fixed by placement at all: ಬ's first word,
  ಬಾ, is inside the very lesson that needs it, and no earlier word contains
  the letter.
- the rest — ದ, ೆ, ಿ, ೇ, ಂ, ಷ, ಣ, ತ, ೋ, ಟ, ೂ — are within-chapter ordering,
  and every one of them is already at the earliest position that still follows
  the word its own lesson cites. Moving them further up would trade a closure
  violation for a forward reference one-for-one.

## Unreleased — the eight letters the track showed but never taught

Kannada asked the reader to decode **eighteen** characters no lesson had ever
introduced. Eight of them were letters, and this tranche teaches them: one
character per lesson, each landing **before** the first lesson that needs it.

  KA-S126  ಉ   ch. 21    KA-S130  ಈ   ch. 31
  KA-S127  ಊ   ch. 23    KA-S131  ◌ೃ  ch. 36
  KA-S128  ಝ   ch. 25    KA-S132  ◌ಃ  ch. 43
  KA-S129  ಥ   ch. 28    KA-S133  ಞ   ch. 47

Measured against `measureScriptClosure`:

- never-taught glyphs **18 → 10**
- closure violations **36 → 30**
- glyphs taught **51 → 59**, script lessons **24 → 32**

The six violations that cleared are the six the placement targeted:
`KA-C27-sanje` and `KA-C30-shubha-sanje` (ಝ), `KA-C29-shubhodaya` and
`KA-C32-iru` (ಉ), `KA-C32-tinnu` (ಊ), and `KA-C39-kaapi` (ಥ). Each had been
printing a letter in load-bearing body text that no earlier lesson had taught.

### Why these eight, and why one every few chapters

Closure is measured in **reading order**, so a letter lesson only pays for the
lessons that follow it. Each of the eight is therefore placed at the last
chapter that still precedes its first consumer, and never two in the same
chapter — chapters 21, 23, 25, 28, 31, 36, 43 and 47, with word lessons already
sitting in the gaps. The writing stays drizzled rather than batched.

None of the eight is taught as a bare shape. Every one is introduced as **the
other half of something the reader already reads**, which is what makes a
one-character lesson cost almost nothing:

- **ಉ, ಊ, ಈ** are the standing forms of the vowel signs ು, ೂ and ೀ, met inside
  *haudu*, *mūru* and *nīvu* — words long since glossed.
- **◌ೃ** closes the same pair from the far side: the reader has had standing ಋ
  since *ṛtu*, and meets its riding hook here.
- **ಝ** and **ಥ** complete the plain-and-breathed run the reader has already
  seen six times (ಕ/ಖ, ಗ/ಘ, ಡ/ಢ, ದ/ಧ, ಪ/ಫ, ಬ/ಭ), so the eighth pair is a pattern
  being finished rather than a fact being added.
- **◌ಃ** is paired with the anusvara ◌ಂ it sits beside in the writing system.
- **ಞ** is taught honestly as a letter the reader will nearly always meet
  stacked, inside ಜ್ಞ, rather than standing.

Two of the eight are marked as rare on purpose. ಝ and ◌ಃ barely occur in native
Kannada words, and the lessons say so instead of implying a frequency they do
not have.

### KA-S125 re-chained

`KA-S125-letter-oo` (ಓ, ch. 34) now sits sixth from the end of the letter chain
rather than last. Its prerequisite moves from `KA-S124-letter-pa` to
`KA-S130-letter-ii`, its warm-up reviews ಈ rather than ಪ fourteen chapters back,
and the claim that it was "the last of the twenty-four" is removed, since it is
no longer either.

### Still open

The remaining ten never-taught glyphs are the Kannada digits ೦–೯, drilled in
both chapter 7 number lessons. They are logged as **HL-C194** rather than fixed
here: both consumers sit in chapter 7, so all ten lessons would have to bunch
into chapters 6–7 with no vocabulary between them, which is the batching this
ramp is built to avoid. The backlog entry proposes splitting chapter 7 into ten
word-plus-digit lessons instead.

## Unreleased — a four-minute first greeting (#12242)

- Rewrite `KA-C01-namaskara` from 333 computed seconds to 240 effective
  seconds while retaining the greeting, left-to-right decode, respectful
  gesture, and `namas` + `kāra` etymology payoff.
- Remove the duplicated script explanation and six-language comparison from the
  learner's first five minutes. Add one finger trace and one guided copy so the
  shorter lesson still begins writing gently.
- Reduce the corpus duration backlog from two lessons to one. Kannada is now at
  zero; the independently logged Telugu lesson remains visible.

## Unreleased — Chapters 60-66: Thirty-five more words, round three

Two rounds took Kannada from 47 pre-A1 headwords to 117, and that was still last
of the seven tracks being carried to the 300-word floor together — spanish 118,
malayalam 119, tamil 119, hindi 120, sanskrit 139, telugu 151. These seven
chapters answer with thirty-five more on the same terms: **one new word per
lesson**, and unlimited reuse of everything already taught.

  60 Animals of the House       ಹಸು ಮೇಕೆ ಕೋಳಿ ಕಾಗೆ ಹಕ್ಕಿ
  61 In the Kitchen             ಮೊಸರು ತುಪ್ಪ ಎಣ್ಣೆ ಬೆಲ್ಲ ಸಕ್ಕರೆ
  62 Things People Make         ನೂಲು ಸೂಜಿ ಹಗ್ಗ ಪೊರಕೆ ಕೊಡೆ
  63 Hunger, Sleep and Health   ಹಸಿವು ನಿದ್ದೆ ಜ್ವರ ನೋವು ಆರೋಗ್ಯ
  64 Joining Words              ಆಮೇಲೆ ಕೂಡ ಇನ್ನೂ ಬೇಗ ಹಾಗಾದರೆ
  65 More Good Manners          ವಿನಂತಿ ಅನುಮತಿ ಸಂಕೋಚ ನಂಬಿಕೆ ಸೌಜನ್ಯ
  66 The Field                  ಹೊಲ ಹುಲ್ಲು ಭತ್ತ ಬೆಳೆ ಸುಗ್ಗಿ

Attainment 117 → 152 of 300, a rise of exactly thirty-five. All seven pre-A1
spine nodes are used a third time, one per chapter, in the same order round two
used them, and the chain runs unbroken from ಬೆಂಕಿ at the end of the weather run
to ಸುಗ್ಗಿ at the end of this one — each chapter's second lesson still practising
the previous chapter's last word.

The ramp got gentler again. R1 falls 0.2664 → 0.2642 with its numerator held at
exactly 1140: not one of the thirty-five new atoms misses its first
reinforcement window, so the whole improvement is a larger denominator.

### The collisions, and which check caught each

Every candidate was checked in four classes: in a lesson body, inside an
existing headword, inside a romanization, and — the one no string search
reaches — inside the GLOSS of a morpheme or a stock phrase.

**Inside an existing headword.** ದಾರ, the obvious word for thread, sits whole
inside ದಾರಿ from the river-and-road chapter. ಕಣ, a threshing floor, sits whole
inside ಕಣ್ಣು from the face chapter, which is why chapter 66 ends on ಸುಗ್ಗಿ
instead. ಸಹ for "also" sits inside ಸಹಾಯ; ಮತ್ತೆ is the first word of the headword
ಮತ್ತೆ ಸಿಗೋಣ; ಎಷ್ಟು is inside the headword ನಿನ್ನ ವಯಸ್ಸು ಎಷ್ಟು?.

**Inside a romanization.** ಕನ್ನಡಿ, a mirror, spells *kannaḍa* inside itself, and
that string is on more pages of this book than any other.

**Inside a gloss — the largest class again, six of them, and none reachable by
grep.** This is HL-C207 holding on a fourth track.

  ಅಕ್ಷರ     "a written character"  all 24 writing lessons say "the single character X"
  ಸ್ವಾಗತ     "welcome"              ಪರವಾಗಿಲ್ಲ has been glossed "you're welcome" since the how-are-you chapter
  ಮಾತ್ರ      "only"                 the replies chapter glosses the emphatic -ē as "only that one"
  ಬೆಳಕು      "light"                the morning lesson glosses beḷaku, "light", outright
  ಬಲ        "strength"             the age lesson glosses vayas as "age, youth, vigor, strength"
  ನಿಧಾನ     "slowly"               every guided-practice block says "it once more, slowly"

ಅಕ್ಷರ is the same shape as sanskrit's *shiilam* and telugu's *aksharam*: the
collision is with a phrase twenty-four files independently use, not with any one
lesson. ಸ್ವಾಗತ is the sharpest of the six, because "welcome" and "you're welcome"
are two different acts wearing one English word, and the reader has had the
second since chapter 3.

ಎಣ್ಣೆ was examined under the same rule and KEPT. The friend-lesson glosses the
Sanskrit *snēha* as "oil, unctuousness, smoothness", but that gloss teaches a
Sanskrit etymon, not a Kannada headword; ಎಣ್ಣೆ is Kannada's own and had never been
on the page. The lesson makes the pair explicit rather than pretending it is not
there. ಪ್ರೀತಿ, "affection", is the word that gloss really does block, and it is not
in this tranche.

Three near-misses were kept and turned into teaching instead of discarded, each
because the two words are genuinely distinct in Kannada letters and only look
close in a romanization: *hakki* against *akki*, ಕೊಡೆ against ಕೊಡು, and ಕೂಡ
against ಕೂದಲು. ಹುಲ್ಲು against ಹಲ್ಲು is the closest pair in the whole book — one
vowel sign — and chapter 66 says so out loud rather than hoping.

### Script, and what the taught-glyph filter cost

Every headword is spelled entirely from glyphs the track's own writing lessons
teach, so the tranche adds no script-closure violation and nothing is laundered
through an exposure-exempt headword: Kannada's exempted-glyph count is unmoved at
162 and its violation count at 37. The same filter shaped the prose — ಉಪ್ಪು is
cited in romanization in the jaggery lesson, because ಉ is a letter no writing
lesson has taught. Forward references hold at 511, rule statements at thirty, and
Kannada's cross-chapter prose references at their pinned 80.

### What the chapters actually do

Chapter 61 is a chain rather than a list: ಹಸು gives ಹಾಲು, ಹಾಲು gives ಮೊಸರು and
ತುಪ್ಪ, and ಎಣ್ಣೆ is the one that comes from a ಬೀಜ instead — so the chapter teaches
where five kitchen words come from, not just what they are.

Chapter 63 teaches one sentence shape five times. ನನಗೆ ಹಸಿವು has exactly the
build of ನನಗೆ ಕನ್ನಡ ಗೊತ್ತು from the knowing lesson, and ನೋವು then attaches to
every body part the book has taught, which is the best trade in the tranche: one
new word, a sentence for each part of the body.

Chapter 66 completes a set three rounds in the making — ಭತ್ತ in the field, ಅಕ್ಕಿ
in the sack, ಅನ್ನ on the plate, one plant under three names that Kannada keeps
strictly apart.

The last lesson deliberately makes no claim about the book ending (HL-C201): it
says these five words end where a farming year does, which stays true however
many chapters come after it.

## Unreleased — Chapters 53-59: Thirty-five more words, round two

The first tranche took Kannada from 47 pre-A1 headwords to 82. That left it last
of the seven tracks that are being carried to the 300-word floor together. These
seven chapters answer with thirty-five more, on the same terms as before: **one
new word per lesson**, and unlimited reuse of everything already taught.

  53 The Sky                    ಆಕಾಶ ಬಿಸಿಲು ಚಂದ್ರ ಚುಕ್ಕಿ ಮೋಡ
  54 The Tree                   ಮರ ಕೊಂಬೆ ಎಲೆ ಬೇರು ಬೀಜ
  55 River and Road             ನದಿ ಕೆರೆ ಬೆಟ್ಟ ಹಳ್ಳಿ ದಾರಿ
  56 In the House               ಚಾಪೆ ಬುಟ್ಟಿ ಚಾಕು ಪಾತ್ರೆ ಪೆಟ್ಟಿಗೆ
  57 More of the Body           ಕುತ್ತಿಗೆ ಬೆನ್ನು ತುಟಿ ಭುಜ ಮೂಳೆ
  58 More Short Replies         ಸ್ವಲ್ಪ ಹೆಚ್ಚು ಕಡಿಮೆ ಬೇಡ ಅಷ್ಟೇ
  59 Weather and Ground         ಗಾಳಿ ಮಂಜು ಮರಳು ಕೆಸರು ಬೆಂಕಿ

Attainment 82 → 117 of 300, a rise of exactly thirty-five. All seven pre-A1
spine nodes are used a second time, one per chapter, and the chain runs unbroken
from ಹೂಮಾಲೆ at the end of the welcome chapter to ಬೆಂಕಿ at the end of this run —
each chapter's second lesson still practising the previous chapter's last word.

The ramp got gentler again rather than steeper. R1 falls 0.2869 → 0.2843 with
its numerator held at exactly 1106: not one of the thirty-five new atoms misses
its first reinforcement window, so the whole improvement is a larger denominator.

Every candidate was checked against the whole track before it was written, in
both directions and inside existing headwords. That check is what kept ಸೂರ್ಯ out
of the sky chapter: the ordinary word for the sun is already on the page, whole,
inside ಸೂರ್ಯೋದಯ in the good-morning lesson, so teaching it here would have made
that earlier page point forwards. ಬಿಸಿಲು took the slot instead, which is the more
useful word anyway — a Kannada sentence about the afternoon is far likelier to be
about the sun's heat than about the sun. ಮಳೆ went the same way, already present
inside ಮಳೆಗಾಲ. The corpus figure for forward references holds at its 500 ceiling
and rule statements hold at thirty.

Every headword is spelled entirely from glyphs the track's own writing lessons
teach, so the tranche adds no script-closure violation and nothing is laundered
through an exposure-exempt headword: Kannada's exempted-glyph count is unmoved at
162. The same filter shaped the prose — ಊಟ appears in romanization rather than in
script, because ಊ is a letter no writing lesson has taught yet.

Two sound laws now carry the run rather than sitting in one remark each. ಬೇರು,
ಬೆಟ್ಟ and ಬೆನ್ನು put Kannada's *b* against the family's *v* in three separate
chapters, and ಹಳ್ಳಿ shows the *p* that became *h* written on road signs the
length of the state, where the unchanged *-palli* survives across the border.

Chapter 59 closes on a deliberate refusal: ಮರಳು has ಮರ standing at the front of
it and is not related to it, and the lesson says so, because knowing when an
etymology is a coincidence is part of what this book is teaching.

Fixed, in `roadmap.md`: the Tamil word cited beside ಬರೆ carried a KANNADA LETTER
RA (U+0CB0) in place of the Tamil one, so a Tamil word was rendering with one
letter drawn from the Kannada font. Both fonts load and nothing was missing, so
the build had always exited 0 with zero warnings while printing the wrong glyph.
Found by decomposing every word in the tree into Unicode letter names from the
FILE BYTES and flagging any word naming two scripts (backlog HL-C202).

## Unreleased — Chapters 46-52: Thirty-five everyday words, one per lesson

Kannada was the furthest behind of every track on the pre-A1 vocabulary floor:
47 headwords against the 300 that floor asks for. These seven chapters answer it
with thirty-five, and with nothing else — **one new word per lesson**, and reuse
of everything already taught, unlimited and on purpose.

  46 Things You Ask For             ಹಣ್ಣು ಬಟ್ಟೆ ದೀಪ ಉಪ್ಪು ಪುಸ್ತಕ
  47 The Leg and the Tooth          ಕಾಲು ಹಲ್ಲು ಕೂದಲು ಬೆರಳು ಹೊಟ್ಟೆ
  48 Naming Who Someone Is          ಶಿಕ್ಷಕ ವಿದ್ಯಾರ್ಥಿ ವೈದ್ಯ ರೈತ ಅತಿಥಿ
  49 Answering With More Than Yes   ನಿಜ ಸಾಕು ಖಂಡಿತ ಬಹುಶಃ ಹಾಗೆ
  50 Taking Your Leave              ಈಗ ನಾಡಿದ್ದು ಪ್ರಯಾಣ ಹೊರಡು ಬೀಳ್ಕೊಡುಗೆ
  51 Courtesy Words                 ಕೃತಜ್ಞತೆ ಉಪಕಾರ ಗೌರವ ಆಶೀರ್ವಾದ ವಂದನೆ
  52 Welcoming a Guest              ಬಾಗಿಲು ಕುರ್ಚಿ ರಂಗೋಲಿ ಹೂವು ಹೂಮಾಲೆ

Each chapter sits on one of the seven pre-A1 spine nodes, five lessons sharing
it, and each lesson chains to the one before — so a word introduced by a
chapter's payoff lesson is still being practised two lessons into the next
chapter. That is why the ramp got *gentler* rather than steeper: the R1
reinforcement ratio falls 0.3064 to 0.3034 even though the corpus grew. Not one
of the thirty-five new atoms misses its R1 window.

Every headword was checked against the whole track before it was written, so
none of the thirty-five re-teaches anything and none of them adds a forward
reference — the corpus figure holds at its 500 ceiling, and so does the
thirty-count on rule statements.

The sound law that separates Kannada from its sisters is the spine of the whole
run rather than a remark in one lesson: ಹಣ್ಣು, ಹಲ್ಲು, ಹೊಟ್ಟೆ, ಹೊರಡು and ಹೂವು all
carry an *h* where the family has a *p*, and ಉಪ್ಪು and ಪುಸ್ತಕ say why two
particular *p* sounds survived — one because it is not at the front of its word,
the other because it walked in from Sanskrit after the law had finished.

Chapters 51 and 52 each close on a word that takes apart in front of the reader:
ಆಶೀರ್ವಾದ carries the same *-vāda* as ಧನ್ಯವಾದ from the first page, and ಹೂಮಾಲೆ is a
Dravidian syllable welded to a Sanskrit one.

Chapters 43, 44 and 45 also move from a bare `unicodeScript` to the track's
`kannada-comparisons` script set, matching chapters 6-42 (backlog HL-C200). Their
rendered output is unchanged, because they happen to cite no cousin script today;
what changes is that a comparison table added to them later can no longer drop
its glyphs silently into the Latin font at exit 0.

## Unreleased — Chapter 41: Pointing, and Asking

Six words and the pattern behind them: ಇದು ಅದು ಇಲ್ಲಿ ಅಲ್ಲಿ ಯಾರು ಎಲ್ಲಿ

Until now the reader could NAME things and not point at them. With these they
can: *this one*, *that one*, *here*, *there*, and the two questions that matter
first — *who?* and *where?* Everything already in the book becomes a sentence
they can use.

The seventh lesson is the reason these six are one chapter. They are not six
words, they are **i- / a- / e-** — three beginnings on the same ending, and changing
the front walks the meaning from near to far to a question. A reader who sees
that once does not have to be taught the third member of the next family they
meet; they will work it out.

The whole chapter is **voice**: nothing in it needs eyes, so it is learnable end
to end at the wheel.

## Unreleased — the first 8 characters this book actually teaches

8 recognition segments, one character each, in chapters 6-13: ನ ◌ು ◌್ ◌ಿ ತ ದ ರ ◌ಾ

Until now this track taught **no letters at all**. Every word was printed in its
own script and the reader had no way in — HL12's measurement put the track at A2
by aspiration and pre-A1 by attainment, with the script strand simply missing.

Each segment names one character, says what it carries, and shows it inside four
words the reader **already says** — so nothing new has to be learned in order to
do the recognising. That is HL12 §2.1's rule made concrete: a lesson may sit at
the frontier of decoding or of meaning, never both, because a reader who fails
one that is new in both cannot tell which one they failed.

They teach recognition and not writing, and that is a sourcing fact rather than a
pedagogical preference. This script has **no cited stroke order** in the corpus —
its own script file says *"Recognition only"* — and HL11 §5 forbids a pen path
without one, because a learner cannot tell an invented stroke order from an
attested one and will drill it for years. So the reader is asked to trace the
printed shape, which needs no source, and the book says plainly that where to
start the character and which way to travel are not written down yet.

Each segment sits **last** in its chapter, after every word in that chapter that
contains its character — so it consolidates rather than pre-teaches, and it costs
the driving edition nothing: `drivablePrefixTotal` is unchanged corpus-wide.

## Unreleased — 38 words a reader can now say

Added `romanization` to 38 lessons that had none, so their headwords become
HL11 *exposure* — something the reader is shown and can use — rather than script
they are stuck on. Each is recovered from the pronunciation the lesson already
gives in its own prose, then checked against the headword's script so a wrong
grab cannot pass. Nothing is transliterated: a mechanical romanization of this
script disagrees with its own authors often enough to teach mispronunciations.

## Chapters 33–34 — The eight verbs, and Kannada as the third Dravidian voice (2026-08-07)

- Added eight lessons under the canonical `VERB-*` tags, in two chapters of
  four. **Chapter 33, "Four Verbs of Mind and Page"**: `KA-C33-yocisu`
  (ಯೋಚಿಸು, `VERB-THINK`), `KA-C33-artha-maadiko` (ಅರ್ಥಮಾಡಿಕೊ,
  `VERB-UNDERSTAND`), `KA-C33-oodu` (ಓದು, `VERB-READ`), `KA-C33-bare` (ಬರೆ,
  `VERB-WRITE`). **Chapter 34, "Four Verbs Between People"**: `KA-C34-tegeduko`
  (ತೆಗೆದುಕೊ, `VERB-TAKE`), `KA-C34-keelu` (ಕೇಳು, `VERB-ASK`),
  `KA-C34-sahaya-maadu` (ಸಹಾಯ ಮಾಡು, `VERB-HELP`), `KA-C34-ishta` (ನನಗೆ ಕನ್ನಡ
  ಇಷ್ಟ, `VERB-LIKE-LOVE`). Sequences 670–740, schema v2, one prerequisite chain
  running out of `KA-C32-gottu`.
- **Two chapters, not one.** Twenty new atoms, ten per chapter, against
  `maxNewAtomsPerChapter: 12` — a single eight-lesson chapter would have
  doubled the budget. Each chapter carries its own `canDo` and its own payoff
  closing over its own four lessons.
- **Coverage.** Kannada goes from **6 to 14** of the shared spine's core forty
  verbs. Each of these eight concepts was already taught by fifteen other
  tracks; Kannada makes each a sixteen-way join, and the third Dravidian voice
  in it after Tamil and Telugu.
- **The Dravidian comparison, now that three sisters are in the corpus.**
  ಬರೆ *bare* is Proto-Dravidian *\*warV-* "to scratch, to draw lines" (DEDR
  5263), the same word as Tamil வரை *varai* and Telugu (వ)రాయు *(v)rāyu*, all
  still meaning **draw** as readily as **write** — and Tamil's *v* against
  Kannada's *b* is the second of the two front-of-the-word laws, set beside
  *p* → *h* (*pattu*/*hattu*, *pasir*/*hasiru*) in one three-column table.
  ಕೇಳು *kēḷu* means **both** "ask" and "hear," because the root is reconstructed
  with both senses (DEDR 2017) and Tamil, Malayalam and Tulu carry the pair;
  Telugu is the sister that split them into *vinu* and *aḍugu*. The lesson
  names the branch split — Kannada, Tamil and Malayalam on one branch, Telugu on
  the next — and then **limits** the claim honestly, because the root reaches
  Telugu's own branch-mates Gondi and Kui: Telugu dropped a word rather than
  never having had one. ತೆಗೆದುಕೊ *tegeduko* cuts the other way, and is allowed
  to: its *tege* files with Telugu *tīyu* (DEDR 3407) and its *koḷḷu* with Tamil
  *koḷ* and Telugu *konu* (DEDR 2151), so Kannada matches **Telugu** on both
  roots and Tamil on only one — descent does not decide every word.
- **The borrowing thread, per word rather than per topic.** ಯೋಚಿಸು is the noun
  ಯೋಚನೆ (Sanskrit योजना *yojanā*, from युज् *yuj* "to yoke" — the root of *yoga*
  and English *yoke*) plus **‑ಇಸು**, the verb-making suffix Chapter 9 already
  used on *kṣamā*; beside it the inherited ನೆನೆ *nene* (DEDR 3683 *\*nen-ay* "to
  think") survives with the job of remembering, which is the very root Tamil
  promoted to its everyday நினை *ninai*. ಓದು runs the other way: it is the
  inherited word (DEDR 1052 *\*ōtu* "to recite, read") left in its ordinary job,
  where Tamil's cognate ஓது narrowed to chanting and everyday reading passed to
  படி *paṭi*. ಸಹಾಯ is Sanskrit सहाय, most probably *saha* "with" + *aya*
  "going" — "one who goes with" — welded to the native ಮಾಡು, a Sanskrit-plus-
  Dravidian hybrid of the same kind Chapter 20's Persian-plus-Sanskrit ಹವಾಮಾನ
  already was.
- **ಇಷ್ಟ, and the claim held down to what is true.** The word is Sanskrit इष्ट,
  past participle of इष् *iṣ*, from the Indo-European root behind English *ask*
  — so the chapter carries two unrelated asking-words. But the **frame** is not
  borrowed: inherited ಬೇಕು *bēku* takes the same dative subject and answers
  Tamil வேண்டும் *vēṇṭum* through the same *v* → *b* law. The cross-language note
  deliberately carries **no census**: a count of how many tracks build liking on
  an experiencer is stale the moment the next tranche lands. The lesson names
  Spanish *me gusta* and Italian *mi piace* beside Tamil *piḍikkum* and says the
  true, permanent thing — Romance and Dravidian borrowed nothing from each other
  here, and arrived at one shape independently.
- **Reinforcement at two cadences, and the orphan count halved twice over.**
  Every lesson practises atoms from the immediately preceding one to three
  lessons, across the chapter seam (`KA-C34-tegeduko` practises Chapter 33's
  *bare*, *ōdu* and *arthamāḍiko*). Each payoff reaches back several chapters:
  `KA-C33-bare` rescues `KA-LEX-C32-BAA-01`, `KA-LEX-C32-TINNU-01`,
  `KA-GRAMMAR-C32-NOODU-02`, `KA-LEX-C07-NUMBERS-6-10-01`,
  `KA-ETYMON-C07-NUMBERS-6-10-02`, `KA-ETYMON-C22-HASIRU-HALADI-01` and
  `KA-LEX-C20-HAVAMANA-02`; `KA-C34-ishta` rescues all three Chapter 6
  dative-subject atoms plus both `KA-C32-gottu` atoms. Measured on the corpus:
  Kannada's never-revisited atoms fall from **20 of 79 to 9 of 99**.
- **Drivability held.** All eight derive `voice`; Chapters 33 and 34 are fully
  drivable, keeping the whole 32–34 verb arc car-safe. No script blocks — the
  canonical `## The letters in this word` heading classifies as a `script`
  block and is **not** detachable, so a lesson carrying one derives `sight`
  (this is why Tamil's and Telugu's own C33–C34 are undrivable). The letters
  each word needs are taught in a `Sounds you'll need` block instead, exactly as
  Chapter 32 does. Every table is three columns at most; the whole-word sight-cue
  scan is clean.
- Wiring: `curriculum.json` gains `KA-PATH-026` on `SPINE-SAY-WHAT-I-DO` with
  the two required extensions `KA-EXT-026-MIND-VERBS` and
  `KA-EXT-027-SOCIAL-VERBS`, and that node's `omits` ledger drops the eight
  concepts now realised (36 → 28). `chapters.json` gains Chapters 33 and 34,
  each payoff assessing **10 of its chapter's 10** introduced atoms (1.00,
  against the 0.5 floor). `core/book-generation.json` gains both targets; the
  generated `ch33-mind-verbs.tex` and `ch34-social-verbs.tex` are `\input` from
  `book.tex`.
- Verified: `tests/integration.test.ts` and `tests/cli.test.ts` 19/19 green;
  `check:modality`, `check:books` and `check:narration` clean; all eight lessons
  under the duration budget (computed 290–299s against the 300s threshold), and
  the track has zero duration violations. The book compiles under XeLaTeX at 120
  pages with **zero** "Missing character" reports; build artefacts removed. A
  throwaway glyph probe first confirmed that Latin Modern Roman itself lacks
  ṁ, ḻ, ṉ, ṟ, ḱ, ʰ, ʼ and the ring-below — the Kannada preamble's
  `
ewunicodechar` fallbacks are what make the first four safe in this book.

## Chapter 32 — The Core Verbs (2026-08-06)

- Added six lessons under the **canonical** `VERB-*` concept tags, the track's
  first: `KA-C32-iru` (ಇರು, `VERB-BE`), `KA-C32-hoogu` (ಹೋಗು, `VERB-GO`),
  `KA-C32-baa` (ಬಾ, `VERB-COME`), `KA-C32-tinnu` (ತಿನ್ನು, `VERB-EAT`),
  `KA-C32-noodu` (ನೋಡು, `VERB-SEE`) and `KA-C32-gottu` (ಗೊತ್ತು, `VERB-KNOW`).
  Sequences 610–660, schema v2, in a single prerequisite chain.
- **Why they were needed.** The track already taught four verbs — *mātanāḍu*,
  *iru*, *hōgu*, *kelasa māḍu* — but every one of them under a Kannada-only tag
  (`KA-VERB-IRU`, `KA-VERB-HOOGU`, …). A namespaced tag joins nothing across
  languages, so on the cross-language measurement Kannada covered **zero** of
  the canonical forty core verbs. It now covers six.
- **One idea per lesson, each on the word that needs it.** The three slots,
  stem + tense + person, so *iruttēne* is literally be + present + I and the
  last bead already means "I" (*iru*). The **p → h law** — Kannada alone among
  the four literary Dravidian languages softened old word-initial \*p- to h-,
  so Tamil *pōgu / pattu / pāl / peyar* answer Kannada *hōgu / hattu / hālu /
  hesaru* while Telugu and Malayalam keep the p, with Old Kannada's surviving ಪ
  spellings as the evidence (*hōgu*). The form you call a verb by against the
  form the beads attach to, *bā* but *baru-*, which finally opens Chapter 4's
  ಹೋಗಿ ಬರುತ್ತೇನೆ — plus Kannada's second initial-consonant habit, \*v- → b-
  (*bā*). The tense bead, and the fact that everyday Kannada keeps **no**
  separate future, leaving *-utt-* to cover both and a word like *nāḷe* to
  settle it (*tinnu*). The person bead, gendered only in the third person, and
  the genuine four-way split in the everyday see-word — *nōḍu*, *pār*, *cūḍu*,
  *kāṇuka* (*nōḍu*). And the closing symmetry: ಗೊತ್ತು is not a verb at all, so
  it has no person slot and the knower must ride outside it in the dative
  (*gottu*).
- **Dravidian discipline held.** No Indo-European cognates invented for native
  Kannada words; every cousin cited is a Dravidian sister with its form
  supplied (Tamil *iru / pōgu / vā / tiṉ / pār / teḷi / aṟi*, Telugu *uṇḍu /
  pōvu / vaccu / tinu / cūḍu / teliyu / telusu*, Malayalam *irikkuka / pōkuka /
  varuka / tinnuka / kāṇuka / aṟiyuka*), and unsettled reconstructions are
  flagged rather than asserted — *nōḍu*'s kinship with Tamil *nōkku* is given
  with the hedge it deserves.
- **Drivability held deliberately.** All six derive `voice`. No script blocks,
  no sight cues, and the two tables are three columns wide at most. The letters
  each word needs are taught inline in its "Sounds you'll need" block, never as
  a gated reading course.
- Wiring: `curriculum.json` gains `KA-PATH-025` on `SPINE-SAY-WHAT-I-DO` — the
  track's first content on that node — with the six lessons attached as the
  required `KA-EXT-025-CORE-VERBS` extension, and that node's `omits` ledger
  drops the six concepts now realised. `chapters.json` gains a Chapter 32 entry
  whose payoff, `KA-C32-gottu`, assesses 7 of the chapter's 12 atoms (0.58,
  above the 0.5 floor). `core/book-generation.json` gains the Chapter 32
  target; the generated `ch32-core-verbs.tex` is `\input` from `book.tex`.
- Verified: integration suite 16/16 green, `check:modality` / `check:books` /
  `check:narration` all clean, and every lesson under the duration budget
  (computed 271–291s against the 300s threshold). The book compiles under
  XeLaTeX with zero "Missing character" reports; build artefacts removed.

## Chapter capability ledger — HL05 (2026-08-06)

- Added [`chapters.json`](./chapters.json), the track's HL05 chapter capability
  ledger. Twenty-six chapters — 6 through 31 — each declare a first-person
  `canDo`, the shared spine nodes they realise, and a `payoff` naming the lesson
  that proves the claim.
- Every `payoff.assesses` list is copied verbatim from its payoff lesson's own
  `practises.knowledge`, so the ledger cannot claim an atom the lesson does not
  actually practise.
- No chapter from 6 on has a terminal `practice`/`practice-mix` lesson — the
  Chapter 5 recap was the last one authored. Each payoff is therefore the
  chapter's **last lesson by `sequence`**, which for these single- and
  double-lesson chapters is also the lesson that recombines the chapter's
  material in its Guided Practice block.
- **Chapters 1–5 are deliberately absent.** Their lessons are still schema v1,
  carry no `practises.knowledge`, and have no `core/book-generation.json`
  target to derive a title from. A payoff written for them would be invented,
  not derived; the gap is left visible as honest debt rather than stubbed.
- Thinnest payoffs, for the representativeness gate that lands next: Chapter 20
  covers 2 of the 4 atoms its two lessons introduce (exactly the 0.5 threshold,
  because the weather lesson closes a chapter that also teaches 11–20), and
  Chapter 6 covers 4 of 6.

## Warning-free complete book (2026-08-03)

- Added explicit static-font faces for every comparison script and readable
  Unicode bookmark fallbacks, eliminating all font-shape and Hyperref warnings.
- Made the five handwritten recap labels unique, shortened only the titles that
  exceeded header or bookmark widths, and added natural page bottoms.
- Adjusted a small set of canonical multilingual examples at durable line-break
  boundaries; regenerated chapters and source hashes remain shared with
  Language Ladder rather than diverging into book-only prose.
- The forced 96-page XeLaTeX build now reports zero missing glyphs, overfull or
  underfull boxes, duplicate destinations, Hyperref warnings, LaTeX warnings,
  and font warnings. All pages were inspected again with no clipping,
  collision, accidental blank page, or leaked schema metadata.

## Canonical Chapters 6–31 in the book (2026-08-03)

- Migrated all thirty Kannada lessons after Chapter 5 to the strict schema-v2
  curriculum contract: canonical spine nodes, unique prerequisite-safe
  sequence, explicit sub-five-minute budgets, typed block boundaries, and
  closed knowledge introductions and assessments.
- Generated twenty-six LaTeX chapters from those canonical lessons instead of
  copying app content into a separate book source. The committed source-hash
  manifest is independently checked against Language Ladder for Chapters 6–31.
- Added a reusable Kannada comparison-font set for Kannada, Tamil, Telugu,
  Malayalam, Devanagari, and Arabic-script examples. The 96-page PDF has zero
  missing glyphs and preserves the full 33-entry top-level chapter outline.
- Rendered and inspected all 96 pages, including the dense case, number,
  calendar, PIE-etymology, and register sections. No clipping, collision,
  accidental blank page, or leaked schema metadata was found.
- The expanded artifact's cleanup baseline is nine overfull boxes, three
  underfull horizontal boxes, seven underfull vertical boxes, four duplicate
  practice labels, 106 Hyperref warnings, and nine font warnings. `HL-B25`
  tracks that publication-hygiene pass separately.
- The single all-books publication gate still compiles all twenty downloadable
  volumes successfully.

## Sub-five-minute lesson remediation (2026-08-02)

- All thirty-seven Kannada duration violations are resolved. Thirty-six lessons
  already computed below five minutes and now declare an honest four-minute
  budget without changing their teaching content.
- The genuinely long Chapter 6 lesson becomes two prerequisite-ordered steps:
  learn **-ಗೆ/-ಿಗೆ/-ಕ್ಕೆ**, **ನಾನು → ನನಗೆ**, and the family *k → g* history;
  then contrast transparent Dravidian suffix stacking with fused Latin endings
  before applying the dative in **ನನಗೆ ಕನ್ನಡ ಗೊತ್ತು**.
- The rewritten suffix lesson computes to 205 seconds and the new stacking
  lesson to 196. The support lesson brings the Kannada track to 60 lessons with
  zero unknown prerequisite ids.
- A forced book build succeeds at 29 pages with no missing glyphs. Canonical
  lessons continue through Chapter 31 while the book stops at Chapter 5
  (`HL-B24`); existing layout, bookmark, duplicate-label, and font warnings are
  tracked in `HL-B25`; roadmap and session-map drift is tracked in `HL-M03`.

## Chapter 6 — Case endings, and the sentence with no subject

- **Chapter 6 authored** (`KA-C06-dative-ge`, `-dative-stacking`,
  `-dative-subject`): the track's first
  **case ending** — reviewing Ch.2/3/5 via `reviews_of`.
- **-ಗೆ** (`KA-C06-dative-ge`): the dative "to/for," taught as the doorway to
  **agglutination**. Kannada **adds** a suffix carrying **one** meaning with the
  **seam visible** (*hesar* + *ige*), where a Latin ending like *-īs* **fuses**
  case+number+declension inseparably. The distinctive Kannada content is a
  **sound-history** point: its **g** is the shared Dravidian dative ***k* softened
  between vowels** — a voicing Kannada carried **into the dative** where its
  sisters kept the hard consonant — while the
  *-kke* form (*kelasakke*) preserves the original hard doubled consonant. Includes
  *nānu* → **ನನಗೆ** *nanage*.
- **Stacking** (`KA-C06-dative-stacking`): the visible Kannada noun-plus-case
  seam and one-suffix/one-job principle now get their own micro-lesson before
  the contrast with Latin *-īs*, which fuses case, number, and declension.
- **ನನಗೆ ಕನ್ನಡ ಗೊತ್ತು** (`KA-C06-dative-subject`): "I know Kannada" — literally
  "**to-me Kannada known**," with **no nominative subject** (contrast Ch.5's *nānu
  kannaḍa mātanāḍuttēne*), and with the further observation that ***gottu* isn't an
  action verb at all** — so nothing in the sentence is *doing* anything. Explains
  the **dative-subject** rule with English's "**methinks**" as the bridge.
- **The Dravidian family thread**, new in this chapter: *-ge / -ukku / -ku / -ikku*
  are visibly the **same suffix** across the four sisters, all building "I know X"
  the same subjectless way.
- Taxonomy: namespaced `KA-CASE-DATIVE`, `KA-DATIVE-SUBJECT`.

## Chapters 3–5 — How-are-you, Farewells, First Verbs

- Three new chapters carry Kannada to Chapter 5, matching the leading tracks' arc.
  One word per lesson, atom-first, Kannada script inline; every root traced
  (`lessons/KA-C0{3,4,5}-*`, `book/chapters/ch0{3,4,5}-*.tex`). Concept tags reuse
  the universal `HL01` taxonomy; verbs namespaced (`KA-VERB-*`). The
  Sanskrit-borrowing-yet-Dravidian-grammar thread runs throughout.
- **Ch. 3 — How Are You**: *hēge* (how; the native *ē-/yā-* questions) → *nīvu
  hēgiddīrā?* (the verb *iru* "to be," the same word Tamil uses) → *nānu* (I ←
  Proto-Dravidian, unrelated to *me*) → *cennāgi* (well ← *cennu* "beautiful";
  *nānu cennāgiddēne* "I'm well") → *paravāgilla* ("no harm" = you're welcome, on
  the Dravidian *illa* shared with Tamil/Malayalam — where Telugu uses *lēdu*) →
  practice.
- **Ch. 4 — Farewells**: *hōgu*/*bā* → *hōgi baruttēne* ("I'll go and come back,"
  tabled across the family) → *nāḷe sigōṇa* (see you tomorrow; *nāḷ* "day" shared
  with Tamil, the "let's ___" *-ōṇa*) → *matte sigōṇa* (we'll meet again; native
  *sigu*, where Tamil borrowed Sanskrit *sandi*) → practice.
- **Ch. 5 — First Verbs**: *mātanāḍu* (← *mātu* "word") → *nānu kannaḍa
  mātanāḍuttēne* (I speak Kannada; no 1st-person gender) → *iru* (to be/stay/live;
  the postposition *-alli*) → *kelasa māḍu* (to work; noun + *māḍu*, the twin of
  Hindi's *karnā*) → practice. Book compiles clean with XeLaTeX (0 missing chars,
  0 undefined refs).

## Chapter 2 — Introducing Yourself

- New chapter around the introduction dialogue (*nanna hesaru … / nimma hesaru
  ēnu?*), atom-first, Kannada inline (`lessons/KA-C02-*`,
  `book/chapters/ch02-introductions.tex`). Every atom traced:
  - **ಹೆಸರು** hesaru ("name") ← Proto-Dravidian *\*pesar*, via Kannada's **p→h**
    shift — cousin of Tamil *peyar*, **not** the Indo-European *name/nām*.
  - **ನನ್ನ** nanna ("my") ← *nānu* ("I").
  - **ನನ್ನ ಹೆಸರು …** — **"my name is…"**; the **zero copula** (no "is").
  - **ನೀನು / ನೀವು** nīnu/nīvu — "you," familiar/respectful; respect by plural.
  - **ಏನು** ēnu ("what") ← Dravidian question-stem *\*yā-/\*e-*.
  - **ನಿಮ್ಮ ಹೆಸರು ಏನು?** — **"what's your name?"**
  - **ಸಂತೋಷ** santōṣa — "pleased to meet you," a **Sanskrit** loan (vs. Tamil's
    native *magiḻcci*) — the Kannada-borrows-Sanskrit thread continued.
  - **practice** — the whole dialogue.
- Book compiles clean with XeLaTeX.

## Chapter 1 — Greetings (Kannada script taught inline)

- New Kannada track on the HL00 framework — the second of the four Dravidian
  tracks, after the Tamil anchor. One word per lesson, slug ids, atom-first,
  derivations shown, LaTeX book. Uses the **vendored** Noto Sans Kannada font
  (loaded by relative `Path=` so local and CI builds match; script shaped via
  `Script=Kannada`, no polyglossia language module needed).
- **No reading course.** Per `HL00`'s inline-letters rule, Kannada is taught
  *inside* each word lesson: a *"The letters in this word"* section introduces
  exactly the letters that word needs, so reading and meaning arrive together.
- Chapter 1 (`lessons/KA-C01-*`), greetings + conversational glue:
  - **ನಮಸ್ಕಾರ** namaskāra ("hello," **Sanskrit** *namas* + *kāra*) — teaches
    the inherent *a*, vowel signs, and the ಸ್ಕ *ottakṣara* (stacked conjunct).
  - **ಧನ್ಯವಾದ** dhanyavāda ("thanks," **Sanskrit** *dhanya* + *vāda*) — the
    aspirated ಧ and the ನ್ಯ conjunct.
  - **ಹೌದು** haudu ("yes," native) — the au/u vowel signs; verb-echo "yes."
  - **ಇಲ್ಲ** illa ("no / isn't," native, root *il*) — cognate with Tamil
    *illai*; negation by a negative existential verb.
  - **ಸರಿ** sari ("okay," native) — the *same word* as Tamil *sari*, one word
    in two scripts.
  - **practice** — recap + the *hōgi baruttēne* / *hōgi banni* farewell (the
    same "go and come back" logic as Tamil's *pōy varugiṟēṉ*).
- The recurring thread: **Kannada borrowed Sanskrit for greetings/politeness
  (namaskāra, dhanyavāda) but kept native Dravidian for the everyday grammar
  (haudu, illa, sari)** — each lesson carries an "Across the family" cognate box
  (English / Sanskrit / Hindi / Tamil / Telugu / Malayalam), every form
  supplied so nothing is assumed. Book compiles clean with XeLaTeX.
