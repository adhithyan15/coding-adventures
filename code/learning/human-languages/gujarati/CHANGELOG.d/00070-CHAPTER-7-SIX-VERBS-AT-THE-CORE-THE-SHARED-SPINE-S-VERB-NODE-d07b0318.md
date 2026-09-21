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

