# Tamil

The sixth track of the [Human Languages](../README.md) curriculum, and the
**anchor** of the four Dravidian tracks (Tamil, Kannada, Telugu, Malayalam), on
the same
framework: one word per lesson, taken apart and traced to its
root; the pieces taught before the whole; and a book you can read straight
through.

## What's different about the Tamil track

- **The script is taught inside the word lessons — no reading course.** Written
  for someone who may not read a single Tamil letter, each word lesson has a
  *"The letters in this word"* section introducing exactly the letters that
  word needs (வணக்கம் brings வ ண க ம, the puḷḷi that removes the inherent
  vowel, and the retroflex ண). A reader who already reads Tamil skims those
  notes. 
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
- **Chapter 3 — How Are You** ([`lessons/TA-C03-*`](./lessons/)): eppaḍi,
  **nīṅgaḷ eppaḍi irukkiṟīrgaḷ?**, nāṉ, nalam, paravāyillai, practice. The verb
  *iru* ("to be") — the copula returns for states; the *iru*/*illai* pair. In
  the book.
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
- **Chapter 32 — The Core Verbs** ([`lessons/TA-C32-*`](./lessons/)): இரு
  (*iru*, be), போ (*pō*, go), வா (*vā*, come), சாப்பிடு (*sāppiḍu*, eat), பார்
  (*pār*, see), தெரி (*teri*, know). The track's first realisation of the shared
  canonical `VERB-*` concepts, so these six join the cross-language corpus
  rather than living under Tamil-only tags. One idea per lesson, all of them
  aimed at the agglutinative machine: stem + tense + person (*iru*), the middle
  bead that alone carries tense (*pō*), a stem with two shapes — *vā* to say it
  by, *varu-* to build on (*vā*) — a verb assembled from a noun plus the light
  verb இடு (*sāppiḍu*), the strong/weak split you can hear in the doubled
  consonant (*pār*), and the one common verb that carries no person at all, so
  the knower moves into the dative (*teri*). In the book, Chapter 32.
- **Writing W01–W04** ([`lessons/TA-W*`](./lessons/)): eight gentle steps teach
  curves, the abugida, retroflexion, the three Tamil n letters, the puḷḷi,
  vowel signs, and whole-word writing for **வணக்கம்** and **நன்றி**.

---

## For contributors

Everything below this line is about how the track is built and checked. It is
here for people working on the curriculum; nothing in it is needed to learn the
language.

## Chapter capabilities

[`chapters.json`](./chapters.json) is the track's
[`HL05`](../../../specs/HL05-chapter-capability-and-step-by-step-shape.md)
capability ledger: for each chapter, one first-person `canDo`, the shared spine
nodes it realises, and a `payoff` naming the lesson that proves the claim, its
kind (`dialogue` / `task` / `production`), a one-line summary, and the knowledge
atoms it exercises. It is **authored intent, not a derived cache** — no
validator may rewrite it.

Two things about this track's ledger are worth knowing before reading it:

- **Chapters 2–5 have no entry, on purpose.** Their lessons are still schema v1
  with no `practises.knowledge`, so a payoff there could not name a single real
  atom. The gap is left visible rather than stubbed, because the HL05 gap
  report's whole job is to measure exactly this kind of debt.
- **Every payoff is a chapter's last lesson by `sequence`,** because no Tamil
  chapter yet ends on a schema-v2 `practice` or `practice-mix`. Where that last
  lesson is one of the eight inline `writing` lessons, or an etymology lesson
  whose closing work is weighing evidence, `payoff.kind` is `task` and the
  summary describes that work plainly instead of dressing it up as an exchange.

The `canDo` statements are pitched at this track's real reader — fluent and
literate in Tamil, but never formally taught its grammar — so they claim
grammatical and etymological precision (which register a moment calls for, why
*I know Tamil* takes a dative experiencer, where a dictionary hedges) rather
than the ability to read the letters.

## Book / fonts

The 31-chapter book compiles with XeLaTeX using **vendored** Noto fonts
(`../../_fonts/`) for Tamil and every comparison script, with no system-font
dependency. Chapters 6–31 are generated from canonical lesson ASTs and checked
against Language Ladder source hashes. `latexmk -xelatex book.tex`.

## Files

- [`lessons/`](./lessons/) · [`chapters.json`](./chapters.json)
  · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `TA-C01-vanakkam`); canonical prerequisite and
sequence metadata governs app/book order. The older roadmap and session map do
not yet enumerate the complete Chapter 31 sequence; that explicit debt is
tracked as `HL-M07` in the shared backlog.
