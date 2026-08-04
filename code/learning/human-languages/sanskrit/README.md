# Sanskrit

A track of the [Human Languages](../README.md) curriculum, on the same
[`HL00`](../../../specs/HL00-human-language-curriculum-framework.md) framework:
one word per lesson, slug ids, atom-first assembly, derivations shown, LaTeX
book.

## What's different about the Sanskrit track

- **A taproot track — like Latin, but pointing east *and* west.** Sanskrit is the
  classical ancestor of the Hindi, Marathi, Punjabi, and Bengali tracks (every
  *namaste* and *dhanyavād* is a worn form of a Sanskrit word met here in full),
  **and** a sister of Latin, Greek, and English. Its roots reach west too — *te*
  ↔ *thee*, *su-* ↔ Greek *eu-*, √gam ↔ *come*, *na* ↔ Latin *nōn* ↔ *no*.
  Learning it ties the two halves of the curriculum together.
- **Devanagari, taught inline** (vendored Noto Sans Devanagari — same font as the
  Hindi/Marathi tracks), with attention to what Sanskrit needs even from a
  Devanagari reader: the *visarga* (ḥ), the vocalic ṛ, conjuncts, and *sandhi*.
- **IAST transliteration** alongside the script, roots traced back toward
  Proto-Indo-European where the trail is clear.

## Progress

- **Chapter 1 — Greetings** ([`lessons/SA-C01-*`](./lessons/)): namaste,
  namaskāraḥ, dhanyavādaḥ, svāgatam, ām/na, practice. In the book.
- **Chapter 2 — Introducing Yourself** ([`lessons/SA-C02-*`](./lessons/)): nāma,
  mama, asti, **mama nāma … asti** (Sanskrit keeps the copula), bhavān/tvam, kim,
  **tava nāma kim?**, ānandaḥ, practice. Each atom a *source* (→ *name/my/is/what*).
  In the book.
- **Chapter 3 — How Are You** ([`lessons/SA-C03-*`](./lessons/)): katham, **bhavān
  katham asti?**, aham (← *ego* → *I*), kuśalam, na cintā, practice. The copula
  trio asmi/asi/asti. In the book.
- **Chapter 4 — Farewells** ([`lessons/SA-C04-*`](./lessons/)): gacchāmi (← *gam*
  → *come*), punaḥ, **punar-darśanāya**, śvaḥ (kept distinct from *hyaḥ*),
  practice. The dative case. In the book.
- **Chapter 5 — First Verbs** ([`lessons/SA-C05-*`](./lessons/)): vadāmi (the
  **dual** *vadāvaḥ*), **ahaṁ saṁskṛtaṁ vadāmi**, vasāmi (← *vas* → *was*),
  karomi (← √kṛ), practice. In the book.
- **Chapter 6 — Numbers 1–5** ([`lessons/SA-C06-*`](./lessons/)): the gendered
  forms first, then an east-west sound-law map, then *pañca* in *Punjab*,
  *pentagon*, and the qualified history of *punch*—three prerequisite-ordered
  micro-lessons.

Chapters 1–6 are in the book. Chapter 6 is generated from the same canonical
schema-v2 lesson AST and source hashes that Language Ladder loads, while the
first five chapters retain their authored long-form narrative during migration.

## Book / fonts

Compiles with XeLaTeX using the **vendored** Noto Sans Devanagari font
(`../../_fonts/NotoSansDevanagari-Static.ttf`). `latexmk -xelatex book.tex`.
Generated Devanagari runs use that font while section bookmarks use the
lessons' Latin romanization.

The forced six-chapter build is warning-free: chapter-qualified recap anchors,
bookmark-safe Devanagari, natural page bottoms, explicit static-font shapes,
and concise running titles keep the downloadable PDF and its outline clean.

## Files

- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `SA-C01-namaste`); order lives in the book and
`session-map.md`.
