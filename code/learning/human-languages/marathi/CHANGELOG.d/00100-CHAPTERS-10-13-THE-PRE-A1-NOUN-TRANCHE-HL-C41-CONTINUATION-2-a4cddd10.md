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

