# Changelog

## Chapters 3–5 — How-are-you, Farewells, First Verbs

- Three new chapters carry Tamil to Chapter 5, matching the leading tracks'
  greet→introduce→how-are-you→farewell→verbs arc. One word per lesson, atom-first,
  Tamil script inline; every root traced (`lessons/TA-C0{3,4,5}-*`,
  `book/chapters/ch0{3,4,5}-*.tex`). Concept tags reuse the universal `HL01`
  taxonomy; verbs namespaced (`TA-VERB-*`). The native-Dravidian-vs-Sanskrit
  thread runs throughout.
- **Ch. 3 — How Are You**: *eppaḍi* (how; the native *e-* question family) →
  *nīṅgaḷ eppaḍi irukkiṟīrgaḷ?* (the verb *iru* "to be" — the copula returns for
  states, where Ch.2's zero-copula couldn't reach) → *nāṉ* (I ← Proto-Dravidian,
  unrelated to *me*) → *nalam* (well ← *nal-* "good," the root of *naṉṟi*) →
  *paravāyillai* ("no harm" = you're welcome; the *iru*/*illai* pair) → practice.
- **Ch. 4 — Farewells**: *pō*/*vā* → *pōy varugiṟēṉ* ("I'll go and come back" —
  the Dravidian promise-of-return goodbye, tabled across Kannada/Telugu/Malayalam
  and the Indo-Aryan tracks) → *nāḷai pārkkalām* (see you tomorrow) → *mīṇḍum
  sandippōm* (we'll meet again; native *mīṇḍum* + Sanskrit *sandi* ← *sandhi*) →
  practice.
- **Ch. 5 — First Verbs**: *pēsu* (stem + tense + person) → *nāṉ tamiḻ pēsugiṟēṉ*
  (I speak Tamil; the signature retroflex *ḻ*; no gender in the 1st person,
  unlike Hindi) → *vāḻ* (to live/flourish) → *vēlai sey* (to work; noun + *sey*,
  the twin of Hindi's *karnā*) → practice. Book compiles clean with XeLaTeX
  (0 missing chars, 0 undefined refs).

## Chapter 2 — Introducing Yourself

- New chapter around the introduction dialogue (*eṉ peyar … / uṅgaḷ peyar
  eṉṉa?*), atom-first, Tamil script inline (`lessons/TA-C02-*`,
  `book/chapters/ch02-introductions.tex`). Every atom is **native Dravidian**
  and traced:
  - **பெயர்** peyar ("name") ← Proto-Dravidian *\*peyar* — pointedly **not** the
    Indo-European *name/nām*; the family fault-line made visible (cognate box
    across all four Dravidian tongues vs. Hindi/English).
  - **என்** eṉ ("my") ← *nāṉ* ("I"); no European cousin.
  - **என் பெயர் …** — **"my name is…"**; introduces the **zero copula** (Tamil
    has no word for "is" in an equational sentence).
  - **நீ / நீங்கள்** nī/nīṅgaḷ — "you," familiar/respectful; respect by the
    plural (the same mechanism as French *vous*).
  - **என்ன** eṉṉa ("what") ← Dravidian question-stem *\*yā-/\*e-*.
  - **உங்கள் பெயர் என்ன?** — **"what's your name?"** (still no "is").
  - **மகிழ்ச்சி** magiḻcci — "pleased to meet you" ("joy"); the rare ழ (*ḻ*).
  - **practice** — the whole dialogue.
- Book compiles clean with XeLaTeX.

## Chapter 1 — Greetings (Tamil script taught inline)

- New Tamil track on the HL00 framework — the **anchor** of the four Dravidian
  tracks. One word per lesson, slug ids, atom-first, derivations shown, LaTeX
  book. Uses the **vendored** Noto Sans Tamil font (loaded by relative `Path=`
  so local and CI builds match).
- **No reading course.** Per `HL00`'s inline-letters rule, Tamil is taught
  *inside* each word lesson: a *"The letters in this word"* section introduces
  exactly the letters that word needs, so reading and meaning arrive together.
  A Tamil script reference page is included in the book as a lookup, explicitly
  not a gated pre-course.
- Chapter 1 (`lessons/TA-C01-*`), greetings + conversational glue:
  - **வணக்கம்** vaṇakkam ("hello," from the native verb *vaṇaṅku*, "to bow") —
    teaches the inherent *a*, the puḷḷi (vowel-killing dot), the retroflex ண.
  - **நன்றி** naṉṟi ("thanks," literally "goodness," from *nal*) — introduces
    Tamil's three-way dental/alveolar/retroflex *n* (and the parallel *l*, *r*
    sets).
  - **ஆம்** ām ("yes") — independent word-initial vowels; verb-echo "yes."
  - **இல்லை** illai ("no / there isn't") — negation carried by a negative verb
    of existence, a deeply Dravidian habit.
  - **சரி** sari ("okay") — one Tamil letter standing for several stop-sounds
    (voicing read from position).
  - **practice** — recap + the *pōy varugiṟēṉ* / *pōy vā* farewell ("go and
    come back," never a bare "I'm leaving").
- The recurring thread: **Tamil's native word-stock vs. its sisters' Sanskrit
  borrowing** — each lesson carries an "Across the family" cognate box
  (English / Sanskrit / Hindi / Kannada / Telugu / Malayalam), every form
  supplied so nothing is assumed. Grounds against English + the Dravidian
  family + Sanskrit. Book compiles clean with XeLaTeX.
