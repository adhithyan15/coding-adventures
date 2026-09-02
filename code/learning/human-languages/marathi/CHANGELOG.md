# Changelog

## Unreleased — chapter 9 is generated, and its nine lessons are typed (HL-C287)

`book/chapters/ch09-introductions.tex` is now generated from its ten lessons.
It was the first of Marathi's four hand-written chapters; three remain (10, 11,
12), and the whole hand-written island is the same shape: 26 schema-v1 lessons
sitting between generated chapters 1–8 and 13–29.

- **The migration was the work.** All nine content lessons were schema v1,
  which declares no knowledge atoms, and `book.ts` refuses to generate from a
  v1 lesson. They now carry v2 frontmatter, an `hl-knowledge` directive on
  every body block, and seventeen atoms between them. Their headings already
  classified — `The letters in this word`, `The word, taken apart`,
  `Grammar Lens:`, `Why it's said this way` — so only four landed on `unknown`
  and each was re-pointed by PREFIXING, never rewriting:
  `The phrase, assembled` → `You'll want to know — the phrase, assembled` (×3),
  `The whole exchange` → `The exchange`, and
  `Atoms banked this chapter` → `What you've built — atoms banked this chapter`.
  A per-character non-ASCII census across all nine shows **zero** Devanagari or
  IAST characters changed; the only deltas are the em dashes those renames add.
- **Two atom ids were minted alongside existing ones rather than re-pointed.**
  `MR-C07-asne` in chapter 14 already owns `MR-LEX-ASNE-AAHE` and
  `MR-GRAMMAR-AAHE-LAST` — for **आहे**, which chapter 9 teaches *first*.
  Duplicate introduction is a hard error, and re-pointing a generated chapter
  has the larger blast radius, so chapter 9 takes `MR-LEX-AAHE-COPULA` and
  `MR-GRAMMAR-VERB-FINAL`. That the corpus teaches the copula twice is real and
  pre-existing; this change makes it visible rather than causing it.
- **`MR-C02-practice` was in no path segment**, which schema v2 forbids for a
  lesson declaring a spine node. It joins `MR-PATH-005` between `MR-C02-anand`
  and `MR-R09-runway-tail-r2` — its sequence position, so reading order is
  unchanged — under a new `MR-EXT-038-INTRODUCTIONS-CONSOLIDATION`, mirroring
  how `MR-C01-practice` is already placed.
- **`chapters.d/0009.json` said the chapter "has no typed knowledge atoms yet
  ... until migration".** That is now false. The payoff names the seventeen
  atoms `MR-C02-practice` actually assesses; at five it sat at 0.28, below the
  0.5 representativeness floor, and it also understated what the recap does.
- **Three things the hand-written chapter taught and the lessons did not say
  are carried back**, found by a content-word census of the two `.tex` files
  rather than by the block gap: the section hook *"a word older than Europe"*,
  which lived in the `.tex` heading and not in the lesson's own title; the
  promise in the runway retrieval that the same six signs come back twice more;
  and — the reverse — the `.tex`'s pointer to "Chapter 4's *punhā bheṭū*",
  which is stale, the lesson's "Chapter 11" being right.
- **Two defects only the generated book could show.**
  `MR-C02-naav` cited PIE **\*h₃nómn̥** with U+0325, COMBINING RING BELOW.
  Latin Modern Roman does not have that glyph — verified by reading the cmap of
  `lmroman10-regular.otf`, not assumed — so it would have reached the reader as
  a missing character. The reconstruction stays; the syllabic *n* is now said
  in words instead of marked with a glyph the book cannot set. And
  `MR-C02-aahe` told the reader it would meet *āhes*/*āhāt* "in Chapter 9"
  while sitting in chapter 9; per HL-C102's own rule the fix is to name the
  thing, so it now says "in the next chapter".

Two pre-flight checks, run before planning and worth repeating for 10-12:

- **The honest denominator is the lessons, not the `.tex`.** German chapter 4
  owned three writing lessons staged separately and appearing nowhere in its
  `.tex`, so the section count understated the chapter by a third.
  `grep -l '^chapter: N$' lessons/*.md` gives 10 / 6 / 6 / 5 for Marathi
  chapters 9-12 against `.tex` section counts of 10 / 6 / 6 / 5 — an exact
  match, so Marathi stages no hidden writing lessons in this island. Confirmed
  rather than assumed.
- **A v1 lesson can carry a glyph that is free hand-written and fatal
  generated.** A character is safe only if it is in the track's own script
  block, in `main-font-charset.json`, mapped by `\newunicodechar` in the
  preamble, **or rewritten by `book.ts`'s escape map** — that last arm is the
  one a naive scan misses, and it covers every arrow, subscript and modifier
  letter chapters 10-12 use. With all four arms applied, chapters 10-12 hold no
  unsettable character; with the escape-map arm removed the same scan still
  flags U+0325, the one that was real.

Counters, each re-measured against the tree:

- Marathi's lesson-content budget moves **179 → 188**. Not one is a new lesson;
  all nine were already written and already in the book. Idioms, senses and
  culture claims are unchanged at 5 / 4 / 7.
- `atomsTaught` **168 → 186**; `atomMeasurementBlindLessons` **28 → 19**.
- `atomChapterSpikes` **0 → 1** — a legitimate RISE. Chapter 9 introduces 17
  atoms against a `maxNewAtomsPerChapter` of 12. No lesson exceeds the per-
  lesson limit of 3; the chapter is genuinely where the whole introduction
  exchange lands. Declaring fewer would mean under-declaring what the lessons
  assess, so the report-only gate records it.
- `reinforcementWindowMisses` **218 → 271**, also a legitimate rise: newly
  declared atoms are newly measured ones, and the debt predates the declaration.
- `atomsNeverRevisited` is back to its pre-existing **1**. It went to 4 first,
  and each of the three was a real gap the recap already closed in prose but
  not in its directives — the *m*→*v* softening, the respectful possessive, and
  the courtesy-by-plural rule are all re-banked by lessons that had not declared
  them.
- `forwardReferences` is unchanged at **4**. It can move in either direction
  when a chapter starts teaching words earlier chapters previewed, so it was
  measured before and after rather than assumed steady.
- Marathi stays in the "chapter debt is already zero" list; it dropped out
  mid-change on payoff representativeness and is back on merit, not by editing
  the list.

Verified: human-language-data 124 test files / 1730 passing; all eleven
`check:*` gates; language-ladder 39 files / 442 passing; the whole Marathi book
compiles under XeLaTeX with zero overfull boxes, and chapter 9's pages were read
as rendered PDF — Devanagari conjuncts, the standalone mātrā, the three-column
exchange table and the atom-bank box all set correctly.

## 2026-09-01 — The A1 exam inventory: borrowing a level, and 88 of 301

- **Marathi now has an A1 target list, and the corpus covers 88 of its 301 points
  (29%).** No awarding body publishes an A1 Marathi syllabus:
  `core/exam-levels.json` records this track as `exam: "no widely-sat ladder"`,
  `basis: "editorial"`. The parallel Hindi effort settled the search negatively
  against the best-placed South Asian candidate — DBHPS publishes examination
  names and prescribed readers and no syllabus, Kendriya Hindi Sansthan and the
  CHTI publish none, and no Council of Europe Reference Level Description exists
  for Hindi. **There is no South Asian equivalent of the Plan Curricular**, and
  the file records `THE SEARCH IS SETTLED, DO NOT REPEAT IT` so no later tranche
  spends hours confirming it.
- **So the file borrows a LEVEL rather than a LANGUAGE.** Spanish is the one
  track here whose A1 point set restates a real awarding body's published
  inventory, which makes its 273 points an *attributable* statement of what an A1
  learner must handle. Each was walked for what it **demands** — "give personal
  information: name, age, origin", "negate a statement", "ask a price" — and the
  Marathi point carrying the same load was written down. `derivedFrom` records
  the mapping per point, so the derivation is auditable line by line: **273 of
  the 301 points derive from Spanish, 28 are Marathi-specific**, and 266 of
  Spanish's 273 points are accounted for with the other 7 listed in
  `proxy.notTransferred` with reasons (three article points — Marathi has no
  articles; two capitalisation points — Devanagari is unicase; two written-accent
  points — Devanagari marks no stress). A test asserts that derivation is
  **total** in both directions, and writing it is what caught `A1-O1-06` going
  missing.
- **Nothing is attributed to anybody.** The `about` disclaims, in those words,
  both the Maharashtra Directorate of Languages and DELE / the Instituto
  Cervantes / the Plan Curricular — because "derived from the DELE A1 inventory"
  is one careless edit away from reading as "DELE says this about Marathi". Every
  Marathi exponent is this project's editorial judgement. Anchors now carry four
  kinds: `sourced-proxy`, `external-framework`, `project-owned`, `editorial`.
- **The rebuild moved the number from 55/131 to 88/301, and both halves are the
  result.** The numerator rose because walking a real inventory found taught
  material an editorially-chosen list had never asked about — evaluative
  notions, mental notions, the colon a form label uses. The denominator nearly
  trebled because it found **twenty thematic domains nobody had enumerated**:
  education, work, leisure, media, housing, services, shopping, health, travel,
  money, government, the arts, religion, the natural world. The corpus covers
  almost none of them. That is not the proxy being unfair; it is what an external
  boundary is for.
- **29% is a FLOOR, not an estimate, and the file says so twice over.** Coverage
  reads *declared* atoms. **SCHEMA-V1:** 26 of the track's 205 lessons — the whole
  of chapters 9 to 12 — declare none while teaching **मी**, **तू / तुम्ही**,
  **माझं**, **काय**, **कसा / कशी / कसं**, **तुमचं नाव काय आहे?**,
  **तुम्ही कसे आहात?**, **काम करणे** and **राहणे**. **EMPTY-INTRODUCES**, which is
  worse because it hides inside schema-v2 where everything *looks* declared: of 66
  v2 lessons carrying `introduces: []`, most are honest retrieval lessons, but
  `MR-R22-request-verbs` and `MR-R23-wellbeing-verbs` drill five polite
  imperatives (**द्या, प्या, आणा, ठेवा, बसा**) and a future (**चालेल**) while
  declaring only the *infinitive* atoms they review, and
  `MR-A1M17-guided-message` / `MR-A1M18-independent-message` teach the guided
  32-word and independent named-reader message — **the A1 writing paper's entire
  second task** — while declaring nothing at all. Thirteen points are marked
  **MEASUREMENT GAP**: taught, unmeasurable, and far cheaper to close than the
  content gaps beside them. The first draft recorded several as "untaught", which
  would have sent an author to rewrite chapters 9 and 24.
- **Every probe names an atom that exists today; nothing is aspirational.** A
  guessed id resolves to "not introduced" forever and is indistinguishable from
  real debt, so an uncovered point is `probe: null` plus a note saying what is
  present and what is missing — `MR-A1-OR-05` records that four of the five
  retroflex letters are taught and **ढ** is not. All four `scope` dimensions stay
  `partial`; a proxy does not close a dimension.
- **What is genuinely missing, ranked by what it unblocks.** The **oblique stem**
  stands between every taught noun and every postposition, so place, time,
  direction and accompaniment are blocked behind one rule. The **ergative -ने** is
  the gate on the whole past tense, not a refinement of it. The **interrogatives**
  कोण, कुठे, कधी are absent in both schemas, and the reading and listening papers
  are wh-item papers. Then the cheapest large win: the **ten Devanagari digits**,
  which no atom teaches and which the writing paper's "complete a practical form"
  needs for a phone number, a house number and a date.
- **`npm run plan` stops calling Marathi unmeasurable.** It now emits
  `exam-point — marathi: cover 213 of 301 A1 exam point(s) marathi does not
  teach`, the corpus-wide backlog moves 190 → 403 across five written
  inventories, and tracks that cannot be measured at all fall 20 → 19. Nothing
  regressed: the 213 were always there and were invisible.
- **Method recorded for the other nineteen editorial tracks** in
  `BACKLOG.d/02610-HL-C290-…`, so this does not have to be rediscovered.

## 2026-09-01 — The second Devanagari runway, and closure closed

- **Script-closure violations 44 -> 0 and never-taught glyphs 7 -> 0.** Corpus-wide
  closure fell 576 -> 532; every one of those forty-four was Marathi's. Of the
  four Devanagari tracks, Marathi and Marwadi now measure clean on both counts;
  Hindi still carries 68 violations and 12 never-taught glyphs, Sanskrit 46
  and 9.
- **The fix was resequencing, not addition, and that is the whole finding.**
  Closure is measured in READING ORDER, so a glyph taught late cannot retire a
  violation that happens early. Seventeen of the twenty-three offending glyphs
  were *already in the corpus* — but the earliest lesson the module could credit
  with teaching them sat at reading position 112, inside the A1 form chapters.
  Marathi did not have a coverage gap; it had an ordering gap, and no amount of
  new material at the end could have touched it.
- **Four new chapters, inserted where the debt starts.** Chapters 5-8 carry
  **twenty-four signs, one per lesson**: the marks **ि ु ू ृ ँ अ**; the velar and
  palatal rows **ख ग घ च छ ज झ**; the retroflex row plus **प** — **ट ठ ड ण प**;
  and **ल श ष उ ऊ ए**. Every later chapter moved +4 (old 5-25 are now 9-29);
  every lesson ID stayed put, so prerequisites, reviews, modality shards and
  assessment references all survived untouched.
- **Order chosen by what each sign unblocks, not by alphabet order.** The marks
  come first because they blocked the most lessons (**ु** alone appeared in 18
  violating lessons, **ि** and **ण** in 15 each). The two consonant rows are
  taught as a *pattern* — plain, breathy, voiced, voiced-and-breathy — so a
  reader who forgets a letter can rebuild it rather than re-memorise it. **ल**
  arrives after **ळ** on purpose: the rarer retroflex was needed first, by
  *kāḷjī* and *ḍoḷā*.
- **Every citation was verified to exist before the lesson was written.**
  HL-C217 refused to author **ग, घ, ख** rather than invent a Wikimedia Commons
  URL to match the house pattern. This tranche queried the Commons API for all
  twenty-four signs first: fifteen consonants have `File:Deva-<glyph>-order.gif`
  by Opiaterein, the independent vowels **अ उ ऊ ए** have
  `File:Devanagari <glyph> stroke order.svg` by Saurmandal, and the five marks
  have neither — so they cite the Unicode Devanagari chart with their
  codepoints, exactly as the track's existing mātrā lessons already do. HL-C217
  is closed by this tranche.
- **The twenty-four new atoms are reviewed at every window.** Each chapter
  closes with an ear-only retrieval payoff that adds no sign, and **nine
  ear-only review lessons** in chapters 9, 13 and 22 carry the set through R2,
  R3 and R4. All twenty-four now hit R1, R2, R3 *and* R4 — zero window misses.
  Marathi's overall missed windows fell R2 71 -> 62, R3 95 -> 80, R4 67 -> 54,
  and its average lessons-per-SCRIPT-atom rose 5.3 -> 6.5, against 5.0 for LEX
  atoms. That answers the corpus-wide finding that script atoms are reviewed
  roughly a fifth as often as vocabulary atoms.
- **The modality cost is recorded, not engineered away.** Twenty-four sign
  lessons are `type: writing`, which `modality.ts` derives as `pen` before it
  looks at any block, so no detachable segment can rescue them. The drivable
  share fell **61% -> 56%** (voice 52 -> 63, sight 52 -> 54, pen 64 -> 88).
  Retyping those lessons to get a voice core out of the derivation would have
  bought the number and lied about the lesson: detach the script block from a
  lesson whose subject *is* the letter and nothing is left. The nine ear-only
  retrieval lessons are where the ear is paid back honestly.
- **One schema-v2 lesson now lives in a legacy hand-written chapter.**
  `MR-R09-runway-tail-r2` had to sit at reading position 71 to land in the R2
  window for chapter 8's last six signs, and that is inside chapter 9, whose
  book text is hand-written. It is declared in `embeddedLessonIds` and carried
  in `ch09-introductions.tex` with a `% canonical-insertion:` marker and a
  `\label{lesson:...}` — the mechanism HL05 already provides, used by Punjabi
  chapter 4 for the same reason.
- **What did not move, deliberately.** Pre-A1 vocabulary is still **48/300**, and
  is now unambiguously the track's only remaining pre-A1 blocker. A vocabulary
  tranche would have moved it to about 60; this tranche bought closure instead,
  because forty-four lessons showing untaught letters is a broken book while
  forty-eight headwords is merely a short one. The eight headwords without
  romanization are still eight and should stay: they are chapter-25 writing
  tasks where withholding the romanization is the exercise.
- Added thirteen sound tags, four chapter capability entries, five curriculum
  path segments and seven extension nodes; regenerated the book, narration,
  modality, gentle-ramp and assessment artifacts; and updated the pinned
  Marathi corpus tests to the new chapter map. Lessons 168 -> 205, chapters
  25 -> 29.

## 2026-08-31 — The pre-A1 verb tranche, and a whole level-gate criterion closed

- Added **four chapters and nineteen lessons** (22-25) carrying **twelve new
  pre-A1 headwords**, every one of them a verb: **देणे, पिणे, आणणे, ठेवणे**
  (asking for an action); **बसणे, वाटणे, झोपणे, चालणे** (a visit and how you
  are); **सांगणे, म्हणणे, शिकणे, मिळणे** (what passes between two people).
  One new headword per lesson, at most three new atoms per lesson, and no
  chapter above nine atoms against the twelve-atom ceiling.
- **Pre-A1 vocabulary moved 36 -> 48** of the 300-headword floor (56 -> 68
  total). Marathi had 149 lessons behind 36 headwords, which is what this
  tranche was aimed at.
- **Closed the HL09 §3.1 verb-vocabulary criterion outright**: 1 distinct
  verb headword at or below pre-A1 against a floor of 5, now 13. The track's
  four earlier verbs were tagged `MR-VERB-*`, which the cross-language join
  cannot see; the new ones carry the canonical `VERB-GIVE`, `VERB-DRINK`,
  `VERB-BRING`, `VERB-PUT`, `VERB-SIT`, `VERB-SLEEP`, `VERB-WALK`, `VERB-SAY`,
  `VERB-LEARN` and `VERB-GET` tags. Missing core concepts fell 33 -> 23.
- **Closed the reinforcement criterion too.** The five pre-A1 atoms revisited
  fewer than twice — **नमस्कार** as a read word, **र**, **बरं**, **येतो/येते**
  and **हृदय** — now get genuine retrieval where the new prose actually uses
  them: **बरं** inside *malā baraṁ vāṭtaṁ*, **येते** inside *malā jhop yete*,
  **नमस्कार**'s halant and **र** reused by **म्हणणे**. Lengthening the track
  also opened the R4 window on eleven older atoms, so chapter 25 carries two
  recognition-at-distance lessons that answer it rather than leaving it.
  **Marathi's only remaining pre-A1 blocker is now vocabulary.**
- **Recorded the romanization that was already there.** Twenty-two chapter 5-8
  lessons carried their romanization inside the gloss (`gloss: name (nāv)`) but
  not in the `romanization` field the exposure rule reads, so their headwords
  measured as load-bearing script. Headwords without romanization fell
  **30 -> 8**, and script-closure violations **51 -> 44**. The eight that remain
  are chapter 21 writing tasks where withholding romanization is the exercise.
- **Rebalanced toward the ear.** Every new word lesson has a voice core with the
  inline-letters section as a detachable segment, so the track's drivable share
  rose **56% -> 61%** while the pen-lesson count did not move at all.
- **Authored one word gloss-first on purpose.** **सांगणे**'s *ga* has never been
  taught, so its Devanagari appears only in the headword (exposure) and its body
  prose is entirely romanized; the lesson says the letter is owed. **म्हणणे**
  then arrives as its fully readable partner. The three missing consonants
  **ग, घ, ख** are filed as HL-C217 with the reason they were not written here:
  no citable stroke-order source was on hand, and inventing one to match the
  house pattern would be worse than a measured gap.
- Declared five concept relocations off `SPINE-NAME-EVERYDAY-ACTIONS` and five
  off `SPINE-SAY-WHAT-I-DO` onto the pre-A1 nodes that now teach them, added
  four curriculum path segments and four extension nodes, and regenerated the
  book, modality, narration and gentle-ramp artifacts. Never-taught glyphs stay
  at 7 and no new closure violation was added.

## 2026-08-26 — Teach the first missing धन्यवाद signs (#13055)

- Added a 12-sign, one-new-shape-at-a-time Devanagari runway for **ः आ भ े ं त
  द ध ब य ळ व**, followed by a supported whole-word **धन्यवाद** writing step.
  Every new lesson is 220 seconds or shorter and cites the script source used
  for its stroke or placement guidance.
- Kept **नमस्कार** and **धन्यवाद** meaning-first until their load-bearing signs
  are taught, while carrying every new sign through visible R1/R2 retrieval,
  no-model R3 retrieval, and distant R4 retrieval.
- Spread writing across observe/trace, guided copy, delayed copy, and dictation;
  regenerated the canonical curriculum, book, narration, modality, progress,
  gentle-ramp, and level-snapshot artifacts through Chapter 18.
- Reduced never-taught Marathi glyphs from 36 to 24 and script-closure
  violations from 60 to 51. The three target opening lessons now have no
  closure violations, with no new duration, ordering, chapter-atom, or payoff
  regression. Remaining script and older-atom reinforcement debt stays visible.

## 2026-08-24 — Split the opening at the chapter atom budget (#12497)

- Split the 20-session opening into a 14-session meaning-and-script chapter
  and a six-session courtesy, response, and leave-taking chapter. The first
  chapter reaches the 12-atom ceiling exactly; the second stays well below it.
- Kept **नमस्कार** meaning-first, preserved the one-sign-at-a-time Devanagari
  runway, and left trace, guided copy, delayed copy, dictation, and independent
  reading in the learner-visible opening. Every lesson remains <=5 minutes.
- Renumbered downstream lesson metadata, capability ledgers, handwritten and
  generated book chapters, narration, modality, progress, and cross-chapter
  prose references through the new Chapter 15 without changing lesson IDs or
  learner sequence.
- Reduced Marathi chapter-level atom spikes from one to zero without raising a
  limit or hiding the remaining script, reinforcement, or migration debt.

## 2026-08-23 — Put writing in learner order and repair family retrieval (#12467)

- Moved the existing 13-step Devanagari runway out of the end of the book and
  directly behind the first greeting, where the curriculum had always intended
  it to run. Chapter/sequence order, prerequisites, reviews, curriculum paths,
  capability ledgers, generated book chapters, and narration now agree.
- Converted the seven legacy opening lessons to atom-explicit schema v2 and
  made the recap test both independent **हो** writing and **नमस्कार** reading.
- Added honest R1, R2, and R3 retrieval for the five Chapter 11 friend/family
  atoms. Every new review is an explicit 135–180 second lesson with learner
  activity; no review credit comes from metadata alone.
- Repurposed Chapter 14 as spaced family-memory work and promoted Chapter 1 to
  canonical generation so lesson, book, narration, and modality artifacts
  cannot silently drift apart again.
- Logged the newly exposed opening-chapter atom budget as #12497 and the
  review-only measurement blind spot as #12496. Neither gate is weakened here.

## Unreleased — complete the pre-A1 writing runway with one word

- Move familiar **हो** through four explicit <=3-minute stages: trace only
  **ह**, guided-copy the two-piece word, hide it for delayed copy, then write
  it from a heard cue with no visible or romanized answer model.
- Add no glyphs in the independent stages and preserve the one-new-shape ceiling
  in the supported stages, so writing difficulty rises without a script jump.
- Put the two new steps into the actual curriculum, book, and narration before
  the next consonant rather than claiming writing-stage credit from metadata.
- Record the five Chapter 11 R3 reviews that become measurable when the track
  grows; #12467 will first reconcile declared and learner-visible order, then
  place those reviews at honest durable distances instead of hiding the debt.
- This proves instruction, not assessment readiness; timed production, mocks,
  calibration, and book-only learner evidence remain separate work.

## 2026-08-21 — Enumerate the pre-A1 assessment task shapes (#12430)

- Turned the project-defined pre-A1 contract into executable reading,
  listening, writing, and speaking parts with exact timing, item, replay, aid,
  and scoring boundaries.
- Preserved four separate 100-point papers and the requirement to reach 60% in
  every skill without aggregate compensation.
- Kept writing productive but gentle: delayed recall, short dictation, and
  bounded independent responses earn points, while tracing and visible copying
  remain instructional supports only.

## [Unreleased]

### Added — pre-A1-to-C2 assessment contract (HL16)

- Added a clearly labelled project-defined Marathi assessment ladder at
  pre-A1, A1, A2, B1, B2, C1, and C2 rather than treating Maharashtra's
  role-specific government-employee examinations as a general proficiency
  certificate.
- Every rung requires independent reading, listening, writing, and speaking
  passes at 60%, with no stronger skill compensating for a weaker one.
- The contract carries Devanagari writing from observe/trace and copying through
  delayed recall, dictation, connected composition, and timed exam production.
- Two timed mocks, rubrics, answer keys, calibration, and book-only human
  validation remain explicit dependencies, so a named destination cannot be
  mistaken for pass-readiness evidence.

## Marathi chapters 1-5 regain their reading order (#12251)

- Add one global, spaced sequence to all 33 legacy lessons, recovered from the
  hand-authored book sections and closed against every prerequisite and review.
- Remove 33 missing-sequence findings plus 15 forward prerequisites and 19
  forward reviews that alphabetical filename fallback had fabricated. Marathi's
  order-integrity backlog moves from 67 defects to zero.
- Keep distinct learner debt honest: forward-language uses move from 12 to 3,
  and one glyph spike disappears in the real order, while the remaining script
  closure and glyph-ramp work stays measurable.

### Added — Chapter 14, the first ten pieces of Devanagari (HL-C216)

Eleven lessons. **Ten teach one piece each; one introduces nothing** and assembles
the greeting from pieces the reader can already write:

    ह  ो              ->  हो "yes"
    न  ा  ी           ->  नाही "no"
    म  स  क  ्  र     ->  नमस्कार, including its conjunct

`scriptLessons` 0 → 11, `taughtGlyphs` 0 → 10, `neverTaughtGlyphs` **46 → 36**.

**The same four ideas as the Gujarati chapter, in the same order** — inherent
vowel, mātrā, virama, conjunct — because they are facts about the abugida rather
than about either language. What changes is the shapes and one signature: Marathi
hangs its letters from the **shirorekhā**, the head-line, and Gujarati is written
by erasing exactly that line. Naming the contrast on the first shape makes both
scripts easier to place.

**Two words readable inside the chapter**: one consonant plus one mātrā gives
*yes* at lesson two, and four pieces give *no* at lesson five.

Two sound facts the script forces and English hides: **क** is the unaspirated *k*
of *skip*, not the breathy *k* of *kit*, and Devanagari writes the difference as
two separate letters; **र** is a light tap, closer to Spanish *pero* than to an
English *r*.


## Chapters 10–13 — the pre-A1 noun tranche (HL-C41 continuation) — 2026-08-08

Thirteen everyday-noun lessons across four new chapters (10–13), continuing
the cross-track pre-A1 vocabulary program and confirming the same measured
mechanism an eleventh-plus time: `vocabularyOf()` counts distinct `headword:`
strings 1:1 with lessons, so thirteen new word lessons move Marathi's pre-A1
vocabulary by exactly thirteen (23 → 36 distinct headwords at or below pre-A1,
shortfall of 300 falling 277 → 264; track-wide 43 → 56).

- **Ch. 10 — Water, Tea, Milk, and Bhakri** (`SPINE-POLITE-REQUEST-REPAIR`,
  previously realized by zero segments — this is the resolution): पाणी, चहा,
  दूध, भाकरी. Builds Marathi's first polite-request pattern, **[word],
  कृपया** — कृपया being Sanskrit's own instrumental of कृपा, "kindness,"
  distinct in construction from Gujarati's करीने-based phrase and Bengali's
  reused imperative, but landing on the same "name it, add the please-word"
  shape. पाणी and दूध are secure Sanskrit/PIE inheritances (*peh₃(i)-* "to
  drink" → potion, poison, symposium; *dʰewgʰ-* → English **doughty**); चहा
  is a Mandarin loan by the overland route, contrasted with English *tea*'s
  sea route; भाकरी never touched classical Sanskrit at all and is not
  Hindi's word for bread (Hindi: रोटी) — traced only to Proto-Indo-Aryan
  *bʰakkaras, "heap, lump," with no further root claimed. The four words
  carry all three Marathi genders (n, m, n, f) in one chapter.
- **Ch. 11 — Friend and Family** (`SPINE-EXCHANGE-NAMES`): मित्र, कुटुंब,
  भाऊ, बहीण. मित्र and कुटुंब are tatsamas, taken up whole from Sanskrit;
  भाऊ and बहीण are tadbhavas, worn down by Prakrit sound change — a split
  visible even in the script, since the tatsama pair carries conjuncts
  (त्र) a learner has to stop for and the tadbhava pair does not. भाऊ is a
  secure PIE cousin of English **brother**; बहीण, deliberately, is **not**
  a cousin of **sister** — Sanskrit भगिनी traces to भज्- "to share,"
  unrelated to PIE *swésōr*. कुटुंब is neuter in Marathi and masculine in
  Hindi, from the identical borrowed Sanskrit word — proof that a
  tatsama's gender is assigned fresh by each receiving language, not
  carried in with the loan.
- **Ch. 12 — Eye, Ear, Mouth, and Nose** (`SPINE-CHECK-WELLBEING`): डोळा,
  कान, तोंड, नाक. डोळा formally teaches the retroflex **ळ** — heard,
  unlabeled, in Chapter 4's *kāḷjī* since the very first wave of this
  track, and taught here as its own atom for the first time. Its root is
  not the expected one: Sanskrit **दोल**, "a swing," not **अक्षि**/**अक्ष**,
  the true inherited Indo-Aryan eye-word (PIE *h₃ekʷ-*, cousin of Latin
  *oculus* and English *eye*) that Hindi's आँख still carries — Marathi
  replaced its old eye-word with a different one entirely. कान continues
  Sanskrit कर्ण through the *same* cluster-simplifying habit Chapter 6
  traced in दोन (rṇ → geminate ṇṇ → ṇ, rather than analogy this time), and
  कर्ण's own root beyond Sanskrit is honestly disputed among scholars —
  named as a dispute, not resolved with an invented answer. तोंड keeps a
  formal Sanskrit doublet, मुख, the same everyday/formal split कुटुंब's
  chapter showed for a household word rather than a body part. नाक alone
  reaches PIE *neh₂s-* / English **nose** without qualification, via
  Prakrit णक्क rather than the more commonly cited नासिका route.
- **Ch. 13 — Heart** (`SPINE-CHECK-WELLBEING`, one lesson): हृदय — the
  surest cognate this track has drawn: PIE *ḱērd-*, behind English
  **heart**, Latin **cor**/**cordis** (**cordial**), and Greek **kardia**
  (**cardiac**), with no metaphorical drift at all. The payoff sets हृदय
  against वाचणे's drift from "to speak" to "to read" (Ch. 8), समजणे's
  drift from "to wake" to "to understand" (Ch. 8), and डोळा's outright
  replacement of the old eye-word (Ch. 12) — a word that, unlike every
  other one this track has taken apart, never drifted.
- **A correction to the brief's premise, checked rather than assumed**:
  the brief expected the avadne-vs-pasand native/Persian-loan contrast
  (Ch. 9) to recur among these nouns. It does not. All thirteen nouns are
  either secure Sanskrit inheritances, one Chinese loanword (चहा), or one
  word with no Sanskrit ancestor at all (भाकरी) — none reach for a
  Persian alternative the way Hindi does. Reported rather than forced.
- Each lesson introduces 2–3 knowledge atoms (at or under
  `maxNewAtomsPerLesson: 3`); chapters total 10/10/10/2 atoms (at or under
  `maxNewAtomsPerChapter: 12`). All four chapter payoffs assess **every**
  atom their own chapter introduces (10/10, 10/10, 10/10, 2/2 —
  representativeness 1.00). Every lesson's `practises.knowledge` reaches
  back to the 1–3 lessons immediately before it; every payoff also reaches
  back further — rescuing `MR-LEX-KHANE` (Ch. 7, thin since authored) via
  bhākarī's "मी भाकरी खातो," and closing every reinforcement gap this
  tranche itself opened (`MR-GRAMMAR-TATSAMA-TADBHAVA-SPLIT`,
  `MR-GRAMMAR-THREE-GENDERS-ONE-CHAPTER`, `MR-GRAMMAR-KUTUMB-NEUTER`,
  `MR-LEX-BAHIN`, `MR-LEX-BHAKARI`, `MR-LEX-DUDH`,
  `MR-SOUND-KARNA-GEMINATION`) before commit. The level gate's
  reinforcement criterion reports **zero** atoms at or below pre-A1
  revisited fewer than twice; continuity's never-revisited count is
  **0 of 0** new atoms left orphaned (down from a mid-authoring peak of 7
  blocking + 3 waived-etymology orphans, all closed).
- Wired via both required steps: `MR-PATH-015..018` path segments plus
  matching `MR-EXT-015..018-LANGUAGE-SPECIFIC` extensions.
  `SPINE-POLITE-REQUEST-REPAIR`'s `omits` ledger still lists
  `COURTESY-PLEASE` (per the curriculum validator's concept-tag matching
  rule — कृपया's lesson carries a language-local `concept_tag`, not the
  canonical `COURTESY-PLEASE` id, the same pattern Gujarati's equivalent
  chapter already established), even though the node is now realized via
  a non-empty path segment.
- Caught and fixed before commit by a forced XeLaTeX compile: Avestan
  (𐬨𐬌𐬚𐬭𐬀) and Old Persian (𐎷𐎡𐎰𐎼) cuneiform in मित्र's etymology, Bengali
  (চা) and Persian (چای) script in चहा's, Gujarati (ભાખરી) and Kannada
  (ಬಕ್ಕರಿ) script in भाकरी's, and the Greek letter theta (θ) in more than
  one place — none covered by this book's fonts, all flattened to plain
  Latin romanizations. Also caught: a combining ring-below (U+0325) inside
  a Proto-Indo-Iranian reconstruction in हृदय's etymology, the exact
  unmapped-diacritic trap a prior wave hit; simplified to go straight from
  Sanskrit to the PIE root, dropping the intermediate reconstruction
  rather than inventing a font fix for it.
- A `MR-C10-bhakari` duration overrun (308s computed against a 280s
  declared ceiling) surfaced only after the font-safety trims changed its
  prompt/sentence count; fixed by tightening two paragraphs rather than
  raising the declared budget past the 300s ceiling.

Verification: forced XeLaTeX build of the 80-page book has zero missing
characters, zero overfull/underfull boxes, zero duplicate labels. All
thirteen lessons compute under the 300s ceiling. `npx vitest run
tests/integration.test.ts tests/cli.test.ts` passes (19/19); `check:modality`,
`check:books`, and `check:narration` all pass. The corpus-wide pinned-number
tests (chapters, continuity, levels, modality-manifest, narration, ramp) shift
with any authored content and are left failing per standing instruction.

## Chapters 8 and 9 — the eight verbs eleven other tracks teach — 2026-08-07

- Authored eight schema-v2 lessons realizing `VERB-THINK`, `VERB-UNDERSTAND`,
  `VERB-READ`, `VERB-WRITE`, `VERB-TAKE`, `VERB-ASK`, `VERB-HELP` and
  `VERB-LIKE-LOVE`: `MR-C08-vichar-karne`, `MR-C08-samajne`, `MR-C08-vachne`,
  `MR-C08-lihine` (sequences 420–450) and `MR-C09-ghene`, `MR-C09-vicharne`,
  `MR-C09-madat-karne`, `MR-C09-avadne` (460–490). Eleven tracks taught these
  eight before this; Marathi is the twelfth.
- **Split across two chapters of four, never one of eight.** Chapter 8
  introduces 8 atoms, Chapter 9 introduces 9, both under the
  `maxNewAtomsPerChapter: 12` budget that one chapter of eight lessons would
  have blown. Each chapter has its own `canDo` and its own payoff closing over
  its own four lessons.
- Gave each lesson one idea, and verified every etymology against sources rather
  than against the brief. **विचार करणे**: वि- on Sanskrit **चर्** *car-*, "to
  move, to range," from Indo-European *\*kʷelh₁-* "to turn" — the root behind
  **wheel** and **cycle**, so a thought is literally something turned over.
  **समजणे**: Prakrit *saṁbujjhaï* ← Sanskrit *sambudhyate*, on **बुध्** *budh-*
  "to wake" — the root that named the **Buddha**, Indo-European *\*bʰewdʰ-*,
  which also gave Greek *punthánomai* and English **bid**. **वाचणे**: Sanskrit
  **वाचयति** *vācayati*, the **causative** of **वच्** *vac-* "to speak," so
  reading is *making something speak*; *vac-* is *\*wekʷ-*, Latin *vōx* and
  *vocāre*, English **voice**, **vocal**, **invoke**, **advocate**. (The
  homonym **वाचणे** "to survive," from *vañcati*, is named and set aside.)
  **लिहिणे**: Sanskrit **लिखति** *likhati*, whose root meant **to scratch** —
  with the internal cousin **रेखा** *rekhā* "a line," and with the external PIE
  cousins reported as **disputed** rather than invented, the lesson resting
  instead on the true typological parallel to Latin *scrībere* and English
  *write*. **घेणे**: Prakrit *gahaï* ← **ग्रह्** *grah-*, Vedic **ग्रभ्**
  *grabh-*, whose *bh* matches English **grab** letter for letter under
  *\*gʰrebʰ-*. **विचारणे**: **विचार** plus *-णे*, so thinking and asking are one
  word in Marathi, where Hindi's **पूछना** is an unrelated root. **मदत करणे**:
  Arabic *madad*, root sense "to stretch out, extend," through Persian — the
  Deccan layer that also gave **माहीत** and **हरकत** — and Marathi spells it
  **मदत** where Hindi and Urdu write **मदद**. **आवडणे**: inherited from Old
  Marathi with cognates in Konkani and Sindhi and, per the sources consulted,
  **no securely established Sanskrit ancestor**, so the lesson claims none —
  a native word where Hindi and Urdu borrowed Persian **पसंद**; beside it
  **प्रेम करणे**, on **प्री** *prī-* ← *\*preyH-*, cousin of English **friend**
  and **free**.
- Made **मला मराठी आवडते** the third sentence on a frame the track already had:
  it stands beside Chapter 7's *malā marāṭhī yete* and Chapter 8's *malā marāṭhī
  samajte* — the experiencer in the dative, the verb agreeing with **मराठी**
  rather than the speaker. Marathi's *āvaḍṇe* is the corpus's seventh
  independent dative-subject "liking," and the first that is **native** rather
  than a loan.
- **Used the canonical `## The letters in this word` heading**, retiring the
  Chapter 7 workaround recorded below. That heading classifies as a `script`
  block, and `script` is a **detachable** block type: full modality for these
  eight lessons is `sight`, but `coreModality` is `voice` for all eight, so both
  chapters report `drivablePrefix` 4 of 4 and the track's drivable percentage is
  unchanged at 98%. Chapter 7's `Sounds you'll need` relabelling was never
  needed; it is left in place rather than migrated in this change, and is
  recorded here as known debt.
- **Reinforced at two cadences.** Every lesson's `practises.knowledge` names
  atoms from the immediately preceding one to three lessons, across the chapter
  seam; each chapter payoff reaches several chapters back. Marathi's
  never-revisited atoms fell from **9 of 19 to 3 of 36**, and the three that
  remain are the three introduced by the final lesson of the track, which no
  later lesson exists to retrieve. Rescued: `MR-GRAMMAR-NE-NEUTER-NOUN`,
  `MR-LEX-PAHNE` and `MR-ETYMON-PASH-SPEK` (in `MR-C08-vichar-karne`);
  `MR-GRAMMAR-DATIVE-KNOWLEDGE`, `MR-LEX-MAHIT-JANNE` and
  `MR-ETYMON-AA-YAA-COME` (in `MR-C08-samajne`); `MR-LEX-NUMBERS-ONE-TO-FIVE`,
  `MR-SCRIPT-DON-FINAL-N` and `MR-SCRIPT-PAACH-NONNASAL` (in `MR-C08-lihine`,
  where two facts about how Marathi *writes* its numbers belong); and
  `MR-ETYMON-DON-ANALOGY`, `MR-ETYMON-PAACH-NASAL-RETENTION` and
  `MR-HISTORY-SELECTIVE-RETENTION` (in `MR-C09-avadne`, where "each language
  chose, word by word" is the argument the chapter is already making).
  `MR-SOUND-CHA-TSAA` is retrieved twice more, because **विचार** and
  **विचारणे** both carry the *chā* that Marathi says nearer *tsā*.
- Added `MR-PATH-013`/`MR-PATH-014` and `MR-EXT-013-MIND-VERBS`/
  `MR-EXT-014-DOING-VERBS` to `curriculum.json`, and dropped all eight concepts
  from the `SPINE-SAY-WHAT-I-DO` omission ledger — which now omits 27, down
  from 35.
- Added Chapter 8 and Chapter 9 to `chapters.json`. Both payoffs assess **every**
  atom their chapter introduces — 8/8 and 9/9, representativeness 1.00 against
  the 0.5 floor — and both were checked against the lessons rather than
  asserted.
- Registered both chapters in `core/book-generation.json` and `book/book.tex`,
  and corrected the preface's claim that later chapters use "sounds you'll
  need," which is now true only of Chapter 7.
- The nine-chapter build is warning-clean: **zero** `Missing character`, zero
  overfull or underfull boxes, zero package warnings. The transliteration set
  (ā ī ū ṇ ṭ ḍ ḷ ś ṣ ñ ṛ) was compiled against Latin Modern Roman before any
  lesson prose was written; the one new Devanagari glyph in the tranche is
  **ड** in **आवडणे**, well inside the three-glyph script-ramp budget.
- No lesson exceeds five minutes: computed effective durations run 232–296
  seconds against the 300-second threshold, and Marathi contributes zero
  duration violations.

## Chapter 7 — the core verbs — 2026-08-06

- Authored six schema-v2 lessons realizing the canonical `VERB-BE`, `VERB-GO`,
  `VERB-COME`, `VERB-EAT`, `VERB-SEE` and `VERB-KNOW` concepts: `MR-C07-asne`,
  `MR-C07-jane`, `MR-C07-yene`, `MR-C07-khane`, `MR-C07-pahne`,
  `MR-C07-mahit-asne` (sequences 360–410). These are the track's first
  realizations of `SPINE-SAY-WHAT-I-DO`, and the first Marathi lessons above A1.
- Gave each lesson exactly one idea. **असणे**: the copula **आहे** closes the
  sentence, and the family behind it (Sanskrit *ásti*, Latin *est*, English
  *is*) — with the honest caveat that *āhe*'s own line of descent is contested,
  some routing it through Middle Indo-Aryan *acchai*. **जाणे**: the present
  ending declares the subject's gender (*jāto / jāte*), which Marathi forces on
  the plainest sentence a beginner can build. **येणे**: *going* and *coming* are
  one Sanskrit verb, *yā-* bare against *ā-yā-* "toward here." **खाणे**: the
  *-णे* infinitive is itself a **neuter** noun, and neuter is the third gender
  Marathi kept where Hindi has two. **पाहणे**: *paś-* is Indo-European *\*spek-*,
  the root behind *spectacle, inspect, species, spy*. **माहीत असणे**: knowledge
  is dative — *malā māhīt āhe*, "to me known is" — with **माहीत** marked as the
  Persian/Arabic Deccan-layer loan it is, beside the inherited **जाणणे** (*jñā-*,
  cousin of English *know*).
- Made the whole chapter **drivable**: six of six lessons derive as `voice`. No
  `script` block, no sight cue, no table wider than three labelled columns. The
  inline letters live in `Sounds you'll need` sections, which is the schema-v2
  home for them; the v1 `The letters in this word` heading derives as `script`
  and would have cost the chapter its drivability.
- Added `MR-PATH-012` and `MR-EXT-012-CORE-VERBS` to `curriculum.json`, dropped
  the six now-realized concepts from the `SPINE-SAY-WHAT-I-DO` omission ledger,
  and registered the generated chapter in `core/book-generation.json`.
- Added the Chapter 7 entry to `chapters.json` with `MR-C07-mahit-asne` as the
  payoff: it assesses 7 of the chapter's 12 introduced atoms (0.58, above the
  0.5 policy floor), because *malā marāṭhī yete* exercises the copula, the
  verb-last rule, subject-gender agreement and the dative knower at once.
- The Chapter 7 build is warning-clean: zero LaTeX or package warnings, zero
  `Missing character`, zero overfull or underfull boxes across the seven-chapter
  book.

## Chapter capability ledger — 2026-08-06

- Added `chapters.json`, the HL05 chapter capability ledger, covering Chapter 6:
  the reader can count *ek, don, tīn, tsār, pāch*, say **चार** with its Marathi
  *ts*, and separate a Marathi innovation from a Hindi retention.
- Made `MR-C06-number-differences` the chapter payoff — the chapter's last
  schema-v2 lesson by sequence (350). It carries the whole argument: **दोन**'s
  final *n* borrowed from the rhyme of *tiṇṇi/doṇṇi*, **पाच** dropping the nasal
  Hindi **पाँच** still writes, and the sound behind an unchanged spelling.
- Recorded `SPINE-COUNT-ONE-TO-FIVE` as the chapter's spine node, matching
  `MR-PATH-011` in `curriculum.json`.
- Omitted Chapters 1–5 rather than stubbing them: all 33 of their lessons are
  schema v1 and declare no `practises.knowledge`, so no payoff there could name
  atoms a lesson actually exercises. Their absence is the debt the HL05 gap
  report exists to measure.
- Measured payoff representativeness for Chapter 6 at 7/7 introduced atoms
  (1.00), comfortably above the 0.5 policy floor.

## Warning-clean six-chapter book — 2026-08-03

- Gave the five handwritten recap sections stable canonical lesson labels rather
  than five copies of `lesson:practice`.
- Kept Devanagari text in PDF bookmarks while removing font-only commands from
  Hyperref strings, and let pages around unbreakable callouts end naturally.
- Declared the vendored static Devanagari file for every requested font shape.
  The forced 31-page build now has zero LaTeX/package warnings, missing glyphs,
  overfull or underfull boxes, and duplicate destinations.

## Generated Chapter 6 — 2026-08-03

- Migrated both numbers 1–5 lessons to the strict schema-v2 contract with stable
  sequence, sub-five-minute duration budgets, typed blocks, and block-level
  knowledge closure.
- Added the shared `SPINE-COUNT-ONE-TO-FIVE` can-do node while keeping Marathi's
  local Devanagari, *don / tīn* analogy, nasal-retention, and *chār / tsār*
  extensions explicit.
- Generated the new LaTeX chapter from the same canonical lesson AST consumed by
  Language Ladder and committed its combined source hash for independent parity
  checks.

## Sub-five-minute remediation — 2026-08-02

- Corrected seven declared five-minute estimates whose computed durations were
  already between 126 and 171 seconds.
- Split the genuinely long numbers lesson into a 163-second counting lesson and
  a prerequisite-ordered 240-second etymology lesson.
- Preserved the complete *don / tīn* analogy, *pāch / pāṁch* retention contrast,
  English *four / five* analogy, and Marathi *chār / tsār* sound shift. The
  shared report now measures zero Marathi duration violations.
- Updated the roadmap and session map to expose both Chapter 6 lesson boundaries.
  Chapter 6's missing one-source book publication remains explicit in the shared
  backlog.

## Chapter 6 — Numbers 1–5, and what "conservative" actually means

- **Chapter 6 authored** (`MR-C06-numbers-1-5`): *ek, don, tīn, chār, pāch* —
  with *chār* noted as pronounced nearer ***tsār***; see the last bullet.
- Marathi and Hindi share a script and an ancestor, so most of these are near
  identical — and the lesson is built on the differences, which turn out to be
  **two different kinds of thing**:
  - **दोन *don*** — an **innovation**, not a retention. The obvious guess is that
    Marathi kept something Hindi lost; it didn't. Sanskrit neuter *trī́ṇi* gave
    Prakrit ***tiṇṇi***, where the **ṇ genuinely belongs to the word for
    three** — the *doubling* being a Prakrit trade for the lost *r*, exactly as
    the Hindi chapter spells out, so the lesson is careful **not** to call the
    whole *-ṇṇi* ancient. The word for *two* was then **reshaped by analogy** to
    match its neighbour in the counting sequence, giving ***doṇṇi***. So
    Marathi's *-n* is a **borrowed
    rhyme taken from the word for "three"**, and Hindi's *do* is simply the form
    that never picked it up. (An earlier draft called it "the worn-down remains
    of an old inflectional ending Marathi held onto" — exactly backwards.) The
    lesson ties it to English *four* getting its *f-* from *five*, which the
    Sanskrit anchor chapter has just taught.
  - **पाच *pāch*** — here it *is* a plain retention difference: Hindi keeps
    Sanskrit *pañca*'s nasal in the chandrabindu, Marathi's spelling drops it.
- **So "neither language is simply older" survives, but sharpened**: the two
  cases aren't even the same kind of event — one is an innovation Marathi
  adopted, the other a retention Hindi made.
- Adds a third difference that is **invisible in the spelling**: Marathi's **च**
  before *ā* is nearer **ts** than English "ch", so चार is *tsār*. The earlier
  "only two differ" was true only of the written forms.

## Chapters 2–5 — introductions, how-are-you, farewells, first verbs

Brings Marathi to Chapter 5 parity with the leading tracks (~26 new deep
lessons + four book chapters), mirroring the Indic template. Every atom traced to
its root; the script stays inline. Book recompiles clean with XeLaTeX (0 missing
characters, 0 undefined references), rasterized and visually QA'd.

- **Chapter 2 — Introducing Yourself** (`lessons/MR-C02-*`): nāv (Sanskrit
  *nāman* → English *name*; the Marathi *m→v* softening), mājhaṁ (three-gender
  agreement), **āhe** (the copula, from *ásti*/√as, and it goes **last**),
  "mājhaṁ nāv … āhe", tū/tumhī (courtesy-by-plural), kāy ("what," the *k-*
  family), "tumchaṁ nāv kāy āhe?", ānand ("joy"), practice.
- **Chapter 3 — How Are You** (`lessons/MR-C03-*`): kasā/kaśī/kasaṁ (gendered
  "how"), "tumhī kase āhāt?", mī (*aham* → Latin *ego*, English *I*), "mī barā
  āhe" (Ch1 *baraṁ* now gendered), **kāhī harkat nāhī** (*harkat* ← Arabic — the
  Deccan Perso-Arabic layer), practice.
- **Chapter 4 — Farewells** (`lessons/MR-C04-*`): punhā (Sanskrit *punar*), bheṭū
  (the "we" in the *-ū* ending), "punhā bheṭū", "udyā bheṭū" (Marathi keeps
  *udyā* tomorrow ≠ *kāl* yesterday), kāḷjī ghyā (the retroflex **ळ**; *ghyā*
  resp. / *ghe* fam.), practice.
- **Chapter 5 — The First Verbs** (`lessons/MR-C05-*`): bolṇe (the *-ṇe*
  infinitive; the **gendered present** *bolto/bolte* — Marathi's signature),
  "mī marāṭhī bolto" (*marāṭhī* ← *Mahārāṣṭra* "great realm"), rāhṇe (postposition
  *-āt* "in"), kām karṇe (√*kṛ* — root of *namaskār*, *karma*; *kām* ← *karma*),
  practice.

Concept tags reuse the universal HL01 ids (WORD-NAME, PRONOUN-MY/I/YOU, WORD-IS,
QUESTION-WHAT/HOW, INTRO-*, STATE-HOW-ARE-YOU, WORD-WELL, COURTESY-YOUREWELCOME,
FAREWELL-LATER/-TOMORROW); verbs and lexemes namespaced (MR-VERB-*, MR-WORD-*,
MR-PHRASE-*). The thread throughout: gender on the **verb** in the present, the
**three** genders, and the extra retroflex letter **ळ**.

## Chapter 1 — Greetings (Devanagari taught inline)

- New Marathi track on the HL00 framework — Indo-Aryan, written in Devanagari
  (reuses the vendored Noto Sans Devanagari font). One word per lesson, slug
  ids, atom-first, derivations shown, LaTeX book. No reading course: the script
  is taught *inside* each word lesson.
- Chapter 1 (`lessons/MR-C01-*`):
  - **नमस्कार** namaskār ("hello/goodbye," Sanskrit *namaḥ* + *kāra*) — Marathi's
    default greeting, where Hindi leans on *namaste*; teaches the halant + स्का
    conjunct.
  - **धन्यवाद** dhanyavād ("thanks," Sanskrit) — the न्य conjunct; warmer
    *ābhārī āhe*.
  - **हो** ho ("yes," distinct from Hindi *hāṁ*).
  - **नाही** nāhī ("no / is not") — on PIE *\*ne* (English *no/not/none*).
  - **बरं** baraṃ ("okay/fine," a native Marathi word) — the anusvāra nasal.
  - **येतो / येते** yeto/yete ("I'll be going," lit. "I come [again]") — the
    Dravidian-style "promise of return" farewell, **gendered on the verb**
    (m./f.).
  - **practice**.
- The recurring thread: what makes Marathi its own language despite sharing
  Devanagari with Hindi — *namaskār*, **three** genders, gender on the verb, and
  the extra letter **ळ** (documented in the appendix). Grounded against English
  + Sanskrit. Book compiles clean with XeLaTeX.
