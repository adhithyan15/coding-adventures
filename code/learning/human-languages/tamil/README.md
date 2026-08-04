# Tamil

The sixth track of the [Human Languages](../README.md) curriculum, and the
**anchor** of the four Dravidian tracks (Tamil, Kannada, Telugu, Malayalam), on
the same [`HL00`](../../../specs/HL00-human-language-curriculum-framework.md)
framework: one word per lesson, slug ids, atom-first assembly, derivations
shown, LaTeX book.

## What's different about the Tamil track

- **The script is taught inside the word lessons — no reading course.** Written
  for someone who may not read a single Tamil letter, each word lesson has a
  *"The letters in this word"* section introducing exactly the letters that
  word needs (வணக்கம் brings வ ண க ம, the puḷḷi that removes the inherent
  vowel, and the retroflex ண). A reader who already reads Tamil skims those
  notes. (Per `HL00`'s inline-letters rule for non-Latin scripts.)
- **Tamil as the Dravidian root the others trace back to.** Tamil is the
  oldest-attested Dravidian language and the most protective of its native
  word-stock. The recurring thread is where Tamil kept a home-grown word
  (*vaṇakkam*, *naṉṟi*) while its sisters Kannada, Telugu, and Malayalam
  borrowed the Sanskrit one — each lesson carries an **"Across the family"**
  cognate box (English / Sanskrit / Hindi / the Dravidian sisters), every form
  supplied so no prior knowledge is assumed.
- **Grammar introduced inline**, where a word first needs it: the three-way
  *n*/*l*/*r* distinction at *naṉṟi*, verb-echo "yes" at *ām*, negation-by-verb
  at *illai*, one-letter-many-sounds at *sari*.

## Progress

- **Chapter 1 — Greetings** ([`lessons/TA-C01-*`](./lessons/)): vaṇakkam →
  naṉṟi → ām → illai → sari → practice (with the *pōy varugiṟēṉ* farewell).
  Tamil script taught inline; Dravidian cognates traced. In the book.
- **Chapter 2 — Introducing Yourself** ([`lessons/TA-C02-*`](./lessons/)):
  peyar, eṉ, **eṉ peyar** ("my name is," zero copula), nī/nīṅgaḷ, eṉṉa,
  **uṅgaḷ peyar eṉṉa?** ("what's your name?"), magiḻcci, practice. Every atom
  native Dravidian and traced (*peyar* ≠ Indo-European *name*); the **zero
  copula** (no word for "is"). In the book.
- **Chapter 3 — How Are You** ([`lessons/TA-C03-*`](./lessons/)): eppaḍi, **nīṅgaḷ
  eppaḍi irukkiṟīrgaḷ?**, nāṉ, nalam, paravāyillai, practice. The verb *iru*
  ("to be") — the copula returns for states; the *iru*/*illai* pair. In the book.
- **Chapter 4 — Farewells** ([`lessons/TA-C04-*`](./lessons/)): pō/vā, **pōy
  varugiṟēṉ** ("I'll go and come back"), nāḷai pārkkalām, mīṇḍum sandippōm,
  practice. The Dravidian promise-of-return goodbye. In the book.
- **Chapter 5 — First Verbs** ([`lessons/TA-C05-*`](./lessons/)): pēsu, **nāṉ
  tamiḻ pēsugiṟēṉ**, vāḻ, vēlai sey, practice. Stem + tense + person; no gender
  in the 1st person. In the book.
- **Chapters 6–31 — Cases, numbers, courtesy, calendar, family, body, food,
  time, weather, animals, colours, and greetings**
  ([`lessons/TA-C{06..31}-*`](./lessons/)): forty-three prerequisite-ordered
  micro-lessons continue the same inline script, grammar, and etymology method.
  All are schema v2, stay below five minutes, and generate twenty-six book
  chapters from the exact canonical sources consumed by Language Ladder.
- **Writing W01–W04** ([`lessons/TA-W*`](./lessons/)): eight gentle steps teach
  curves, the abugida, retroflexion, the three Tamil n letters, the puḷḷi, vowel
  signs, and whole-word writing for **வணக்கம்** and **நன்றி**. They are
  dependency-ordered schema-v2 companions embedded inside Chapter 1 rather than
  a gated alphabet course.

## Book / fonts

The 31-chapter book compiles with XeLaTeX using **vendored** Noto fonts
(`../../_fonts/`) for Tamil and every comparison script, with no system-font
dependency. Chapters 6–31 are generated from canonical lesson ASTs and checked
against Language Ladder source hashes. `latexmk -xelatex book.tex`.

## Files

- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `TA-C01-vanakkam`); canonical prerequisite and
sequence metadata governs app/book order. The older roadmap and session map do
not yet enumerate the complete Chapter 31 sequence; that explicit debt is
tracked as `HL-M07` in the shared backlog.
