# Arabic

The fourth track of the [Human Languages](../README.md) curriculum, on the
same [`HL00`](../../../specs/HL00-human-language-curriculum-framework.md)
framework: one word per lesson, slug ids, atom-first assembly, derivations
shown, LaTeX book.

## What's different about the Arabic track

Arabic doesn't *trace* to roots — its roots are on the **surface**. Nearly
every word is built from a **three-consonant root** carrying a core meaning,
poured into fixed patterns (s-l-m → *salām*/*islām*/*muslim*/*salaam*), which
is the whole curriculum's obsession made literal. So the Arabic track teaches
the **root system** itself as the organizing engine.

Two more things:

- **The script is taught inside the word lessons — no reading course.** Written
  for someone who may not read a single Arabic letter, each word lesson has a
  *"The letters in this word"* section introducing exactly the letters that word
  needs, right to left (سلام brings ا ل م س and the long-ā; مرحبا adds ب ر ح). A
  reader who already reads Arabic skims those notes. (Per `HL00`'s inline-letters
  rule for non-Latin scripts.)
- **Grounded against English + Spanish.** Arabic's long shadow over Spanish is
  a recurring thread: the article **al-** smuggled into English *algebra*/
  *alcohol*, and the sun-letter assimilation you can still hear in Spanish
  *azúcar* (← *as-sukkar*) — every form supplied so no prior Spanish is assumed.
  The Al-Andalus loanwords the Spanish track traces *backward* are met here from
  the source.

## Progress

- **Chapter 1 — Greetings** ([`lessons/AR-C01-*`](./lessons/)): salām →
  marḥaban → al- → as-salāmu ʿalaykum → ṣabāḥ al-khayr → masāʾ al-khayr →
  shukran → practice. The Arabic script is taught inline (RTL, connecting
  letters, dots-on-a-skeleton, the emphatic consonants, ʿayn/hamza), and the
  root engine + attached *al-* are shown as the words are built. In the book.
- **Chapter 2 — Introducing Yourself** ([`lessons/AR-C02-*`](./lessons/)): ism,
  -ī ("my"), **ismī** ("my name is," zero copula), anta/anti (gendered "you"),
  mā, **mā ismuka/ismuki?** ("what's your name?"), tasharrafnā, practice. The
  zero copula (shared with Dravidian) and "you" split by **gender**. In the
  book.
- **Chapters 3–7 — Wellbeing, farewells, and courtesy**
  ([`lessons/AR-C0{3,4,5,6,7}-*`](./lessons/)): ask and answer how someone is,
  assemble **maʿa s-salāma**, respond yes or no, and add please and sorry. Six
  dependency-ordered writing companions introduce only the letters and joins
  needed by the spoken lessons. In the book from canonical schema-v2 lessons.
- **Chapters 8–18 — Calendar and everyday domains**
  ([`lessons/AR-C{08..18}-*`](./lessons/)): days, colours, family, body, seasons,
  food, months, dayparts, clock time, age, and weather, with Arabic's root and
  pattern system kept visible. In the book from canonical schema-v2 lessons.
- **Chapters 19–27 — Counting, description, and leave-taking**
  ([`lessons/AR-C{19..27}-*`](./lessons/)): numbers one through twenty, animals,
  more colours, **ʿafwan**, and a gentle sequence from tomorrow and day/night
  vocabulary to **tuṣbiḥ ʿalā khayr**. In the book from canonical schema-v2
  lessons.

All forty-five lessons in Chapters 3–27 remain below five effective minutes.
Their twenty-five generated chapters carry the same source hashes Language
Ladder recomputes from the browser-loaded lesson AST, so app and book cannot
drift silently.

## Book / fonts

The book compiles with XeLaTeX using **vendored** Noto Naskh Arabic and Noto
Sans Hebrew fonts (`../../_fonts/`), loaded by relative path — so Arabic script
and Semitic comparisons build identically locally and in CI, with no system-font
dependency. `latexmk -xelatex book.tex`.

The complete 104-page artifact builds with zero missing glyphs, layout,
duplicate-destination, bookmark, LaTeX, or font-shape warnings. Open-right blank
versos are intentionally retained for print and contain no running header or
page number.

## Chapter capabilities

[`chapters.json`](./chapters.json) is the track's
[`HL05`](../../../specs/HL05-chapter-capability-and-step-by-step-shape.md)
capability ledger. Each entry says, in the reader's own first-person words, what
finishing that chapter lets them do, and names the lesson that proves it:

```json
{
  "chapter": 17,
  "title": "Asking Someone's Age",
  "canDo": "I can ask how old someone is in Arabic and give my own age without using a verb.",
  "payoff": { "lesson": "AR-C17-kam-umruka", "kind": "dialogue", "assesses": ["…"] }
}
```

The file is **authored intent**, not a derived cache — no validator may rewrite
it. Chapters 3–27 are covered. Chapters 1 and 2 are deliberately left out: their
recap lessons are still schema v1 with no declared knowledge atoms, so a payoff
there could only be invented. That absence is honest, measurable debt and is
reported as such.

## Files

- [`lessons/`](./lessons/) · [`chapters.json`](./chapters.json)
  · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `AR-C01-salam`); schema-v2 lessons carry their
prerequisite-safe sequence in canonical metadata consumed by both app and book.
