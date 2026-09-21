## Chapters 35–40: pre-A1 vocabulary depth — family, face, heart, tea and a meal (2026-08-08)

- **Telugu's pre-A1 vocabulary count was 33 distinct headwords against a 300-word
  target, not because the track lacked content but because its earlier family,
  body, and food chapters taught several words per lesson under one shared
  `headword:` field.** `vocabularyOf()` counts distinct headword strings 1:1 with
  lessons, so six family words in `TE-C12-kutumbam` counted as **one**. This
  tranche adds thirteen new one-headword-per-lesson lessons; pre-A1 vocabulary
  moves from **33 to 46**, and the track's total (any level) from **66 to 79**.
- **Telugu's seven pre-A1 spine nodes were already fully realized before this
  tranche** — `SPINE-MEET-GREET`, `SPINE-COURTESY-THANK`, `SPINE-RESPOND-BASIC`,
  `SPINE-EXCHANGE-NAMES`, `SPINE-CHECK-WELLBEING`, `SPINE-POLITE-REQUEST-REPAIR`,
  and `SPINE-TAKE-LEAVE` all have segments in `curriculum.json`, and the level
  gate reports no `spine-nodes` blocker for Telugu at pre-A1. There was no
  spine-node gap to close; this tranche is pure vocabulary depth.
- Adds **Chapter 35 — Family and Friends** (`TE-PATH-028`, sequences 750–760):
  `TE-C35-kutumbam` (కుటుంబం, the Sanskrit *tatsama* collective noun none of
  Chapter 12's six specific-person words supplied) and `TE-C35-snehitudu`
  (స్నేహితుడు, the everyday Sanskrit-derived word for "friend," set beside the
  native DEDR-attested నేస్తం, *nēstaṁ*).
- Adds **Chapter 36 — Son and Daughter, Two Different Roots** (`TE-PATH-029`,
  sequences 770–780): `TE-C36-koduku` (కొడుకు, "son," Proto-Dravidian *\*kōẓ-*,
  cognate not with "daughter" but with కోడలు, "daughter-in-law") and
  `TE-C36-kuuturu` (కూతురు, "daughter," a wholly separate Proto-Dravidian root
  *\*kūnttu*, cognate with Kannada ಕೂಸು *kūsu*, "infant, maiden"). Telugu does
  **not** build son/daughter on one shared root the way Kannada's
  *magu/maga/magalu* does — the honest, Telugu-specific finding, corrected from
  this brief's working assumption that the Dravidian family tranches would share
  that structure.
- Adds **Chapter 37 — Four Words on the Face** (`TE-PATH-030`, sequences
  790–820): `TE-C37-kannu` (కన్ను, "eye" — closing a loop Chapter 32's *cūḍu*
  lesson left open, having named the word without ever teaching it),
  `TE-C37-cevi` (చెవి, "ear," a near-exact match with Tamil செவி), `TE-C37-mukku`
  (ముక్కు, "nose," a regular Proto-Dravidian reflex flagged irregular in the
  comparative record, likely contaminated by the separate root *\*mok-*, "face"),
  and `TE-C37-noru` (నోరు, "mouth" — the one face word with **no** confirmed
  source for its own root; unlike Tamil, Malayalam, and Kannada, which all share
  one *vāy*/*bāyi* root, నోరు shows no visible trace of it, and nothing is
  invented to fill the gap).
- Adds **Chapter 38 — The Heart, Two Ways** (`TE-PATH-031`, sequences 830–840):
  `TE-C38-gunde` (గుండె, the everyday native word, Proto-Dravidian *\*kuṇṭV*,
  cognate with Kannada's own native ಗುಂಡಿಗೆ *guṇḍige*) and `TE-C38-hrudayam`
  (హృదయం, the literary Sanskrit loan, a genuine Proto-Indo-European cousin of
  English *heart*, Latin *cor*/*cordis*, and Greek *kardía*, all from PIE
  *\*ḱērd-*).
- Adds **Chapter 39 — Tea and Milk, Two Roads** (`TE-PATH-032`, sequences
  850–860): `TE-C39-tii` (టీ, "tea" — a direct English loan whose own root
  travelled by **sea**, Hokkien *tê* through Malay *teh*, the opposite road from
  the **overland** *chai* — Mandarin *chá* through Persian *chāy* — that Hindi,
  Kannada, and Marathi carry) and `TE-C39-paalu` (పాలు, "milk," Proto-Dravidian
  *\*pāl* unchanged, matching Tamil *pāl* and Malayalam *pāl* exactly; here
  Kannada ಹಾಲು *hālu* is the odd one out, via its own regular *p*-to-*h* law,
  not Telugu).
- Adds **Chapter 40 — A Meal** (`TE-PATH-033`, sequence 870): `TE-C40-bhojanam`
  (భోజనం, "a meal" — Sanskrit *bhoja/bhuj*, "to enjoy, to eat," a word Chapter
  32's *tinu* lesson already named as native తిండి's polite counterpart without
  ever teaching it; this lesson closes that second loop the way Chapter 37's
  కన్ను closed the first).
- **Reinforcement discipline, both directions.** Every new lesson's
  `practises.knowledge` reaches back into the preceding one to three lessons
  (closing R1/R2 windows), and each chapter's last lesson closes its own
  chapter's atoms for the HL05 payoff. Seven atoms that were revisited fewer
  than twice — `TE-LEX-C08-DAYACHESI-01`, `TE-PRAGMATICS-C08-DAYACHESI-03`,
  `TE-SCRIPT-C08-DAYACHESI-04` (rescued in `TE-C39-tii`, the natural "please"
  callback for a drink request), `TE-LEX-C12-KUTUMBAM-01` (rescued in
  `TE-C35-kutumbam`), `TE-LEX-C13-SHARIRA-BHAGALU-01` (rescued in
  `TE-C37-kannu`), `TE-GRAMMAR-C19-VAYASU-02` (rescued in `TE-C36-koduku`), and
  `TE-PRAGMATICS-C09-KSHAMINCHANDI-03` (rescued in `TE-C40-bhojanam`, apologising
  for a late meal) — are now revisited at least twice, distributed thematically
  across the tranche rather than dumped in a single capstone lesson. A second
  pass caught six of this tranche's **own** new atoms sitting at only one
  revisit each (`TE-LEX-C35-KUTUMBAM-01`, `TE-LEX-C35-SNEHITUDU-01`,
  `TE-LEX-C36-KODUKU-01`, `TE-LEX-C36-KUUTURU-01`, `TE-LEX-C37-MUKKU-01`,
  `TE-LEX-C37-NORU-01`); each now gets a second reach-back one lesson further
  down the chain. Telugu's pre-A1 reinforcement blocker is now **fully
  closed** — zero atoms at or below pre-A1 are revisited fewer than twice.
- **Font/etymology corrections made against this brief's assumptions.** Tamil's
  word for "buttocks/pit," குண்டி (*kuṇṭi*), was dropped from the heart
  etymology note rather than cited as a semantic cognate of గుండె, because its
  modern sense could not be verified as the Wiktionary-cited Proto-Dravidian
  cognate's original meaning — the lesson cites only the safely attested Telugu/
  Kannada pair instead. Chapter 38's vocalic-*r* note distinguishes the
  *independent* letter ఋ (Chapter 14's ఋతువు) from the *dependent* vowel sign
  ృ (హృదయం), rather than calling them "the same mark." No English
  Proto-Indo-European cousin is claimed for భోజనం (Sanskrit *bhuj*) — that
  etymology is left at its Sanskrit root, the same discipline the corpus already
  applies to అర్థం.
- Wiring: six new `curriculum.json` path segments (`TE-PATH-028`–`TE-PATH-033`)
  on `SPINE-EXCHANGE-NAMES` (×2), `SPINE-CHECK-WELLBEING` (×2), and
  `SPINE-POLITE-REQUEST-REPAIR` (×2), each with a matching required
  `TE-EXT-029`–`TE-EXT-034` extension; the three spine nodes' `segments` ledgers
  updated to match (their `omits` ledgers are unchanged, since every new
  `concept_tag` is Telugu-namespaced and matches no canonical HL01 concept — the
  same convention Chapters 12/13/15/19 already use). Six new `chapters.json`
  entries, each with a `canDo`, a `payoff`, and an `assesses` list copied
  verbatim from its payoff lesson's own `practises.knowledge`. Six new
  `core/book-generation.json` targets and six new `book.tex` inputs.
- **Chapter 8's pre-existing atom-budget violation (`TE-C08-dayachesi`, 4 atoms
  against a 3 budget) is untouched** — it predates this tranche and sits outside
  its scope; fixing it was explicitly out of scope for this pass.
- **Verified.** `npm run build` clean. `check:modality`, `check:books`, and
  `check:narration` all clean after regeneration. `npx vitest run
  tests/integration.test.ts tests/cli.test.ts` — 19/19 green. The HL05 chapter
  gates report zero findings for chapters 35–40. The book compiles under
  XeLaTeX to 150 pages with **zero** `Missing character` lines and no undefined
  references; a stray romanization typo (Telugu glyphs bleeding into an
  italicized romanization for స్నేహితురాలు) was caught by that compile and
  fixed. Build artifacts removed before commit.
- **Corpus-snapshot pins in `modality-manifest`, `levels`, `chapters`,
  `continuity`, `narration`, and `ramp` tests are DELIBERATELY left failing.**
  This is wave 5 of the pre-A1 vocabulary program; Telugu was authored on its
  own branch in parallel with the other wave-5 tracks, and only the merged
  numbers are the real ones — re-pinning here alone would repeat the mistake
  the verbs test's own comment already records.

