# Changelog

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
