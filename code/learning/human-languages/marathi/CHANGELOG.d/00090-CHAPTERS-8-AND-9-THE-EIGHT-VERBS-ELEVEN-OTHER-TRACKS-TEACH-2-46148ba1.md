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

