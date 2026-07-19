# Changelog

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
