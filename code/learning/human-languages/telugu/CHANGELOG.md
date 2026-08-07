# Changelog

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

## Handwriting: the gap named, not filled — HL-C41 (2026-08-06)

- **No Telugu handwriting was authored, and that is the deliberate outcome.** The
  track still teaches zero letter formation: `data/scripts/telugu.json` has 0 of 455
  entries with a `strokeOrder`, and there are no `type: writing` lessons. Tamil has
  11/11 and Devanagari 28/28, so this remains a real gap in three of twenty tracks
  (Kannada 0/455 and Malayalam 0/468 are identical).
- **The blocker is provenance.** `strokes.ts` admits a letter only with a citation
  and a URL for its stroke ORDER — the pen path's shape is checked against the
  vendored font, but no font records the order, so it must trace to a published
  source. Not one such source could be opened for a single Telugu letter. Zero
  letters were authored rather than any uncited ones. The full search record is in
  [`BACKLOG.md`](../BACKLOG.md), *Findings from HL-C41*.
- **"Telugu is written without lifting the hand" is a simplification.** Telugu's
  roundness does make many letters loop-continuous, which is genuinely teachable, but
  the published statement about Telugu stroke direction is that the order is *not*
  uniform — clockwise for some letters, counter-clockwise for others — and the
  `talakattu` tick crowning most consonants is described as its own mark. So
  `penLifts` stays **absent** for every Telugu entry, which means NOT VERIFIED.
- **`telugu.json` now states the rule it is governed by.** Its `notes` record that
  only base consonants and vowel signs are ever authored (a syllable's figure is
  assembled from its parts), that `penLifts` absent means NOT VERIFIED and never
  none, and that it must never be inferred from `strokeOrder.length`. The rule is
  expanded in [`data/scripts/README.md`](../data/scripts/README.md). Authoring 455
  syllables was never the work; authoring ~36 base shapes is.
- No lesson content changed. Chapter counts, book output, and the track's 78%
  drivable figure are untouched.

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
  chapter's **last lesson by `sequence`**. For Chapter 31 that is the register
  lesson `TE-C31-subha-madhyahnam-register`, which is the right payoff anyway:
  the chapter's promise is knowing *when* the greeting fits, not just saying it.
- **Chapters 1–5 are deliberately absent.** Their lessons are still schema v1,
  carry no `practises.knowledge`, and have no `core/book-generation.json`
  target to derive a title from. A payoff written for them would be invented,
  not derived; the gap is left visible as honest debt rather than stubbed.
- Thinnest payoff, for the representativeness gate that lands next: Chapter 20
  covers 2 of the 4 atoms its two lessons introduce — exactly the 0.5 threshold,
  because the weather lesson closes a chapter that also teaches 11–20.

## Warning-free 95-page book (2026-08-03)

- Explicit static font faces and bookmark-safe script commands remove all font
  substitution and Hyperref warnings across Telugu and its five comparison
  scripts without dropping any inline examples.
- Chapter-specific legacy practice labels and natural `\raggedbottom` page
  endings remove duplicate destinations and underfull-page warnings.
- Concise headings, a responsive family table, a reflowed traditional-month
  list, and a shorter Chapter 20 title remove every remaining overfull line.
  The full vocabulary, grammar, comparison, and etymology content remains in the
  lesson bodies shared with Language Ladder.
- The forced 95-page XeLaTeX build now reports zero missing glyphs, box warnings,
  duplicate destinations, Hyperref warnings, LaTeX warnings, or font warnings.
  All pages and the complete 93-entry outline were inspected; a visual-only
  running-header collision found during that review is fixed as well.

## Canonical Chapters 6–31 publication (2026-08-03)

- Thirty later-track lessons now use schema v2 with explicit shared-spine
  placement, prerequisite-safe sequences, typed knowledge boundaries, honest
  sub-five-minute budgets, skills, modes, strands, register, and variety.
- Twenty-six generated chapters extend the downloadable book through Chapter
  31. Their source hashes and lesson ids are independently reproduced by
  Language Ladder, keeping book and app content synchronized.
- A reusable multi-script generator set selects the right vendored font for
  Telugu, Tamil, Kannada, Malayalam, Devanagari, and Arabic-script comparisons.
  The 95-page forced XeLaTeX build has zero missing glyphs; every page and the
  complete outline were inspected.
- The expanded book's remaining layout, duplicate-label, bookmark, and font
  warnings are recorded in `HL-B23`; Telugu roadmap/session-map reconciliation,
  including Chapter 20's numbers-and-weather grouping, remains in `HL-M02`.

## Sub-five-minute lesson remediation (2026-08-02)

- All thirty-six Telugu duration violations are resolved. Thirty-five lessons
  already computed below five minutes and now declare an honest four-minute
  budget without changing their teaching content.
- The genuinely long Chapter 31 lesson becomes two prerequisite-ordered steps:
  build **శుభ మధ్యాహ్నం** from the widened “noon” word shared with Kannada,
  then distinguish the two-source formal-register claim from the one-source
  lower-frequency claim. They compute to 152 and 193 seconds.
- The new support lesson brings the Telugu track to 60 lessons with zero unknown
  prerequisite ids.
- A forced book build succeeds at 29 pages with no missing glyphs. Canonical
  lessons continue through Chapter 31 while the book stops at Chapter 5
  (`HL-B22`); existing layout, bookmark, duplicate-label, and font warnings are
  tracked in `HL-B23`; roadmap and session-map drift is tracked in `HL-M02`.

## Chapter 6 — Case endings, and the sentence with no subject

- **Chapter 6 authored** (`TE-C06-dative-ku`, `-dative-subject`): the track's first
  **case ending** — reviewing Ch.2/3/4/5 via `reviews_of`.
- **-కు/-కి** (`TE-C06-dative-ku`): the dative "to/for," taught as the doorway to
  **agglutination**. Telugu **adds** a suffix carrying **one** meaning, keeping its
  shape with the **seam visible** (*pēru* + *ku*), where a Latin ending like *-īs*
  **fuses** case+number+declension into one indivisible lump; a four-row table sets
  the systems side by side. Notes that *-ku* and *-ki* are **one suffix adjusting
  to the preceding vowel**, and includes the pronoun shift *nēnu* → **నాకు** *nāku*.
- **నాకు తెలుగు వచ్చు** (`TE-C06-dative-subject`): "I know Telugu" — literally
  "**to-me Telugu COMES**." Two payoffs at once: there is **no nominative subject**
  (contrast Ch.5's *nēnu telugu māṭlāḍatānu*), and the verb is **వచ్చు**, the very
  "to come" taught in Ch.4 — a language you know is a thing that *comes to you*.
  Explains the **dative-subject** rule (knowing, liking, wanting *happen to* you)
  with English's "**methinks**" as the bridge.
- **The Dravidian family thread**, new in this chapter: *-ku / -ukku / -ge / -ikku*
  are visibly the **same suffix** across the four sisters, all of which build "I
  know X" the same subjectless way.
- Taxonomy: namespaced `TE-CASE-DATIVE`, `TE-DATIVE-SUBJECT`.

## Chapters 3–5 — How-are-you, Farewells, First Verbs

- Three new chapters carry Telugu to Chapter 5, matching the leading tracks' arc.
  One word per lesson, atom-first, Telugu script inline; every root traced
  (`lessons/TE-C0{3,4,5}-*`, `book/chapters/ch0{3,4,5}-*.tex`). Concept tags reuse
  the universal `HL01` taxonomy; verbs namespaced (`TE-VERB-*`). Telugu's
  heavy-Sanskrit-borrowing-yet-Dravidian-grammar character runs throughout.
- **Ch. 3 — How Are You**: *elā* (how; the native *e-* questions) → *mīru elā
  unnāru?* (the verb *uṇḍu* "to be") → *nēnu* (I ← Proto-Dravidian, unrelated to
  *me*) → *bāgā* (well; *nēnu bāgunnānu* "I'm well") → *paravālēdu* ("no harm" =
  you're welcome, built on Telugu's own *lēdu* — where Tamil/Kannada/Malayalam
  use *illa*) → practice.
- **Ch. 4 — Farewells**: *veḷḷu*/*vaccu* → *veḷḷi vastānu* ("I'll go and come
  back," tabled across the Dravidian family) → *rēpu kaluddām* (see you tomorrow;
  the "let's ___" *-ddām*) → *maḷḷī kaluddām* (we'll meet again; native *kalu*,
  where Tamil borrowed Sanskrit *sandi*) → practice.
- **Ch. 5 — First Verbs**: *māṭlāḍu* (← *māṭa* "word"; stem + tense + person) →
  *nēnu telugu māṭlāḍatānu* (I speak Telugu — "the Italian of the East"; no
  1st-person gender) → *uṇḍu* (to be/stay/live; the postposition *-lō*) → *pani
  cēyu* (to work; noun + *cēyu*, the twin of Hindi's *karnā*) → practice. Book
  compiles clean with XeLaTeX (0 missing chars, 0 undefined refs).

## Chapter 2 — Introducing Yourself

- New chapter around the introduction dialogue (*nā pēru … / mī pēru ēmiṭi?*),
  atom-first, Telugu inline (`lessons/TE-C02-*`,
  `book/chapters/ch02-introductions.tex`). Every atom traced:
  - **పేరు** pēru ("name") ← Proto-Dravidian *\*pēr* — twin of Tamil *peyar*,
    **not** the Indo-European *name/nām* (even Sanskrit-heavy Telugu kept the
    native word).
  - **నా** nā ("my") ← *nēnu* ("I").
  - **నా పేరు …** — **"my name is…"**; the **zero copula** (no "is").
  - **నువ్వు / మీరు** nuvvu/mīru — "you," familiar/respectful; respect by plural.
  - **ఏమిటి** ēmiṭi ("what") ← Dravidian question-stem *\*yā-/\*e-*.
  - **మీ పేరు ఏమిటి?** — **"what's your name?"**
  - **సంతోషం** santōṣam — "pleased to meet you," a **Sanskrit** loan (as in
    Kannada; vs. Tamil's native *magiḻcci*).
  - **practice** — the whole dialogue.
- Example names are invented (Mira / Arun), not reused from any source text.
  Book compiles clean with XeLaTeX.

## Chapter 1 — Greetings (Telugu script taught inline)

- New Telugu track on the HL00 framework — the third of the four Dravidian
  tracks. One word per lesson, slug ids, atom-first, derivations shown, LaTeX
  book. Uses the **vendored** Noto Sans Telugu font (relative `Path=`, shaped
  via `Script=Telugu`, no polyglossia language module needed).
- **No reading course.** Per `HL00`'s inline-letters rule, Telugu is taught
  *inside* each word lesson.
- Chapter 1 (`lessons/TE-C01-*`), greetings + conversational glue:
  - **నమస్కారం** namaskāram ("hello," **Sanskrit** namas + kāra) — inherent
    *a*, the talakaṭṭu, vowel signs, the స్క below-stacking conjunct, and the
    anusvāra ం.
  - **ధన్యవాదములు** dhanyavādamulu ("thanks," **Sanskrit** stem + Telugu plural
    *-mulu*) — the aspirated ధ, న్య conjunct, and a first look at Dravidian
    agglutination.
  - **అవును** avunu ("yes," native) — yes/no as statements of being.
  - **లేదు** lēdu ("no / there isn't," native) — Telugu's *different* root
    (*lē-* / *kā-*), where its sisters use *il-*; the existence-vs-identity
    split (*lēdu* / *kādu*).
  - **సరే** sarē ("okay," native) — the family word *sari* in Telugu dress.
  - **practice** — recap + the *veḷḷi vastānu* / *veḷḷi raṇḍi* farewell (same
    "go and come back" logic as Tamil and Kannada).
- The recurring thread: **Sanskrit for greetings/politeness, native Dravidian
  for the everyday grammar** — plus Telugu's own twist, its divergent "no."
  Each lesson carries an "Across the family" cognate box (English / Sanskrit /
  Hindi / Tamil / Kannada / Malayalam), every form supplied so nothing is
  assumed. Book compiles clean with XeLaTeX.
