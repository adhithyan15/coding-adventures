## Chapter 32: the core verbs, under canonical tags (2026-08-06)

- **Telugu taught 60 lessons across 31 chapters and four verbs — *māṭlāḍu*,
  *cēyu*, *uṇḍu*, *veḷḷu*/*vaccu* — every one of them under a Telugu-only tag
  (`TE-VERB-MATLADU`, `TE-VERB-CEYU`, `TE-VERB-UNDU`, `TE-VERB-VELLU`).** A
  namespaced tag joins nothing across languages, so on the cross-language
  measurement the track covered **zero** of the canonical forty core verbs.
- Adds **Chapter 32 — The Core Verbs**: six lessons, one verb each, in a single
  prerequisite chain — `TE-C32-undu` (ఉండు, `VERB-BE`), `TE-C32-vellu` (వెళ్ళు,
  `VERB-GO`), `TE-C32-raa` (రా, `VERB-COME`), `TE-C32-tinu` (తిను, `VERB-EAT`),
  `TE-C32-cuudu` (చూడు, `VERB-SEE`), `TE-C32-telusu` (తెలుసు, `VERB-KNOW`).
  Sequences 610–660, all schema v2. Telugu now covers **6 of the core 40**.
- **One idea per lesson, and each is a place Telugu differs from its sisters
  rather than a place it agrees.** The three slots on Telugu's own be-verb, plus
  the fact that *uṇḍu* **cannot be negated** — Telugu switches to the separate
  verb *lē-*, the root already met as Chapter 1's *lēdu*, and now conjugable
  (*lēnu*, *lēḍu*, *lēdu*), where Tamil, Kannada and Malayalam all negate on the
  shared *il-* (undu). One tense-piece covering both "I go" and "I will go",
  because Telugu merged habit and future where Tamil keeps them apart — and
  "right now" rebuilt out of *unnā-*, so *veḷtunnānu* literally contains the
  previous lesson's be-verb (vellu). The command-shape against the suffixing
  stem, *rā* but *vaccu-*, which is what Chapter 4's *veḷḷi vastānu* has been
  carrying all along (raa). The inherited eat-root Telugu **kept** where Tamil
  demoted its *tiṉ* and assembled *sāppiḍu* — with the honest register split
  beside it: the verb stayed Dravidian while the polite mealtime nouns
  (*bhōjanaṁ*, *āhāraṁ*) are Sanskrit *tatsama* loans (tinu). Four sisters, four
  unrelated see-verbs but one shared eye (*kannu* · *kaṇ* · *kaṇṇu* · *kaṇṇ*),
  and the respectful **-అండి** of *kūrcōṇḍi*/*kṣamin̄caṇḍi* generalised into a
  slot every stem can fill — *cūḍaṇḍi*, *raṇḍi*, *tinaṇḍi*, *veḷḷaṇḍi* (cuudu).
  And the closing asymmetry: *telusu* has **no person-ending at all**, so the
  knower rides in the dative, which finally lets Chapter 6's *nāku telugu vaccu*
  be separated from *nāku telusu* — *vaccu* marks a **skill**, *telusu* a
  **fact** (telusu).
- **Dravidian discipline held.** No Indo-European cognate was invented for any
  Telugu word; every cousin cited is a Dravidian sister with its form supplied
  (Tamil *iru*/*pō*/*vā*/*tiṉ*/*pār*/*teri*, Kannada *iru*/*hōgu*/*bā*/*tinnu*/
  *nōḍu*/*gottu*, Malayalam *irikkuka*/*pōkuka*/*varuka*/*tinnuka*/*kāṇuka*/
  *aṟiyām*), Sanskrit words are marked as loans, and unsettled roots are flagged:
  *veḷḷu*'s own deeper history is explicitly left open rather than guessed at.
- **Drivability held.** All six derive `voice`. No script blocks, no sight cues,
  and the three tables are three wide. The letters each word needs are taught
  inline in a **"Sounds you'll need"** block — the schema-v2 spelling of the
  track's *"The letters in this word"* section, which has no v2 block type; the
  track's other v2 spelling, *"Script you'll notice"*, would have derived `sight`
  and cost the chapter its drivability. Every letter used had already appeared in
  an earlier chapter, so nothing new was gated behind reading.
- **Wiring.** `curriculum.json` gains `TE-PATH-026` on `SPINE-SAY-WHAT-I-DO` —
  the track's first content above A1 — with the six lessons attached as the
  required `TE-EXT-026-CORE-VERBS` extension, and that node's `omits` ledger
  drops the six concepts now realised (`VERB-INFINITIVE` and
  `VERB-PRESENT-HABITUAL` stay omitted, because they are). `chapters.json` gains
  a Chapter 32 entry whose payoff, `TE-C32-telusu`, assesses 7 of the chapter's
  12 atoms (0.58, above the 0.5 floor) and fires no chapter-gate finding.
  `core/book-generation.json` gains the Chapter 32 target; the generated
  `ch32-core-verbs.tex` is `\input` from `book.tex`.
- **Verified.** Integration suite 16/16 green; `check:modality`, `check:books`
  and `check:narration` all clean; every lesson under the duration budget
  (computed 279–295s against the 300s threshold). The book compiles under
  XeLaTeX with zero `Missing character` reports; build artefacts removed.
- **Corpus pins in `modality-manifest`, `levels`, `verbs`, `chapters` and
  `narration` tests are DELIBERATELY left failing.** Telugu was authored in
  parallel with other tracks and only the merged numbers are real; re-pinning
  here alone would repeat the mistake the verbs test's own comment records.

