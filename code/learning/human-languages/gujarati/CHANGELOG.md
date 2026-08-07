# Changelog

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
