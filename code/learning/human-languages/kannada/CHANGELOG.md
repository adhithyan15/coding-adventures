# Changelog

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
