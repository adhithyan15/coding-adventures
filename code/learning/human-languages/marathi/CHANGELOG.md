# Changelog

## Marathi chapters 1-5 regain their reading order (#12251)

- Add one global, spaced sequence to all 33 legacy lessons, recovered from the
  hand-authored book sections and closed against every prerequisite and review.
- Remove 33 missing-sequence findings plus 15 forward prerequisites and 19
  forward reviews that alphabetical filename fallback had fabricated. Marathi's
  order-integrity backlog moves from 67 defects to zero.
- Keep distinct learner debt honest: forward-language uses move from 12 to 3,
  and one glyph spike disappears in the real order, while the remaining script
  closure and glyph-ramp work stays measurable.

## [Unreleased]

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
