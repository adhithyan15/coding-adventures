# Telugu

The eighth track of the [Human Languages](../README.md) curriculum, the third
of the four Dravidian tracks (after [Tamil](../tamil/README.md) the anchor and
[Kannada](../kannada/README.md)), on the same
[`HL00`](../../../specs/HL00-human-language-curriculum-framework.md) framework:
one word per lesson, slug ids, atom-first assembly, derivations shown, LaTeX
book.

## What's different about the Telugu track

- **The script is taught inside the word lessons — no reading course.** Each
  word lesson has a *"The letters in this word"* section introducing exactly
  the letters that word needs (నమస్కారం brings the vowel signs, the స్క
  below-stacking conjunct, and the anusvāra ం). A reader who already reads
  Telugu skims those notes.
- **Telugu as the Sanskritised Dravidian — and the family's odd one out on
  "no."** Like Kannada, Telugu borrowed heavily from Sanskrit (*namaskāram*,
  *dhanyavādamulu*) yet keeps native Dravidian for the everyday grammar. But
  where Tamil, Kannada, and Malayalam all say "no" on the root *il-*, Telugu
  goes its own way with *lēdu* / *kādu* — a reminder the family has real
  branches. Each lesson carries an **"Across the family"** cognate box, every
  form supplied so nothing is assumed.
- **Grammar introduced inline**: agglutination (the plural *-mulu*) at
  *dhanyavādamulu*, yes/no as statements of being at *avunu*, the
  existence-vs-identity split (*lēdu*/*kādu*) at *lēdu*.

## Progress

- **Chapter 1 — Greetings** ([`lessons/TE-C01-*`](./lessons/)): namaskāram →
  dhanyavādamulu → avunu → lēdu → sarē → practice (with the *veḷḷi vastānu*
  farewell). Telugu script taught inline; Dravidian cognates traced. In the
  book.
- **Chapter 2 — Introducing Yourself** ([`lessons/TE-C02-*`](./lessons/)):
  peru, naa, **nā pēru** ("my name is," zero copula), nuvvu/mīru, ēmiṭi,
  **mī pēru ēmiṭi?** ("what's your name?"), santōṣam, practice. Every atom
  traced (*pēru* ← *\*pēr*, twin of Tamil *peyar*; *santōṣam* Sanskrit). In the
  book.
- **Chapter 3 — How Are You** ([`lessons/TE-C03-*`](./lessons/)): elā, **mīru elā
  unnāru?**, nēnu, bāgā, paravālēdu, practice. The verb *uṇḍu* ("to be"); Telugu's
  own *lēdu* where its sisters use *illa*. In the book.
- **Chapter 4 — Farewells** ([`lessons/TE-C04-*`](./lessons/)): veḷḷu/vaccu,
  **veḷḷi vastānu** ("I'll go and come back"), rēpu kaluddām, maḷḷī kaluddām,
  practice. The Dravidian promise-of-return goodbye. In the book.
- **Chapter 5 — First Verbs** ([`lessons/TE-C05-*`](./lessons/)): māṭlāḍu, **nēnu
  telugu māṭlāḍatānu**, uṇḍu, pani cēyu, practice. Stem + tense + person; no
  1st-person gender. In the book.
- **Chapters 6–31 — Cases, numbers, courtesy, calendar, family, body, food,
  time, weather, animals, colours, and greetings**
  ([`lessons/TE-C{06..31}-*`](./lessons/)): thirty prerequisite-ordered
  micro-lessons continue the same inline script, grammar, and etymology method.
  All are schema v2, stay below five minutes, and generate twenty-six book
  chapters from the exact canonical sources consumed by Language Ladder.

## Chapter capabilities

[`chapters.json`](./chapters.json) is the track's
[`HL05`](../../../specs/HL05-chapter-capability-and-step-by-step-shape.md)
capability ledger: for each chapter, one first-person `canDo` ("I can tell
someone the time on the hour in Telugu, in the singular and the plural"), the
shared spine nodes it realises, and the `payoff` lesson that proves the claim,
with the exact knowledge atoms that payoff exercises.

Chapters **6–31** are authored — twenty-six entries. Chapters **1–5 are absent
on purpose**: their lessons are still schema v1 with no `practises.knowledge`
and no `core/book-generation.json` target, so a payoff for them could only be
invented. That absence is measurable debt, not a placeholder.

Because no chapter after 5 has a terminal `practice` lesson, each payoff is the
chapter's last lesson by `sequence`. Chapter 31 is the one place where that rule
picks a `culture` lesson rather than a word or phrase lesson — and correctly so,
since that chapter's promise is judging when శుభ మధ్యాహ్నం fits the setting.

## Book / fonts

The book compiles with XeLaTeX using the **vendored** Noto Sans Telugu font
(`../../_fonts/`), loaded by relative path — so it builds identically locally
and in CI, no system-font dependency. A forced build produces 95 visually
inspected pages with zero missing glyphs, layout warnings, duplicate
destinations, bookmark warnings, or font warnings.
`latexmk -xelatex book.tex`.

## Files

- [`lessons/`](./lessons/) · [`chapters.json`](./chapters.json)
  · [`curriculum.json`](./curriculum.json)
  · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `TE-C01-namaskaram`); order lives in the book and
canonical prerequisite metadata. The roadmap and authoritative session map
currently stop before the full Chapter 31 sequence; that explicit metadata debt
is tracked as `HL-M02` in the shared backlog.
