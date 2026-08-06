# Russian

A track of the [Human Languages](../README.md) curriculum — the first on the
**Cyrillic** script —, built the same way as:
one word per lesson, taken apart and traced to its root; the script taught
inside the words that need it; grammar introduced only when a word needs it.

## What's different about the Russian track

- **Cyrillic taught inside the words — no reading course.** Each lesson has a
  *"The letters in this word"* section introducing exactly the letters that
  word needs. The track's spine is the **four false friends** — в=v, р=r, с=s,
  н=n — the letters that look Latin and lie; by the end of Chapter 1 they're
  fixed. Full per-letter decomposition lives in
  [`data/scripts/cyrillic.json`](../data/scripts/cyrillic.json); the track
  points at it via [`track.json`](./track.json).
- **English cousins through deep Indo-European roots.** Russian is Slavic —
  Indo-European, like English — so its oldest words rhyme with words you own:
  *нет* is the ancient negation **\*ne** (English *no, not, never*); *привет*
  shares its "speak" root with **Soviet** (*совет*, a council); *есть* "is" is
  the same verb as English *is*.
- **Courtesy words as fossilised prayers.** *спасибо* = *спаси Бог* ("God save
  you"), a sibling of Spanish *adiós* and English *goodbye*; *пожалуйста* asks
  for a favour and doubles as "you're welcome."
- **Grammar inline**: the formal/informal split and *politeness = plural* rule
  arrive at *здравствуйте* (its polite *-те*).

## Progress

- **Chapter 1 — Greetings & courtesy** ([`lessons/RU-C01-*`](./lessons/)):
  привет (hi) → здравствуйте (hello, formal) → спасибо (thank you) → да (yes) →
  нет (no) → пожалуйста (please / you're welcome), plus a practice recap. Six
  words, and enough Cyrillic to read them all cold.
- **Chapter 2 — Introducing yourself** ([`lessons/RU-C02-*`](./lessons/)):
  я → ты / вы → why вы is polite → меня зовут… → как вас зовут? → why Russian
  asks “how” → очень приятно, followed by three focused practices for the
  exchange, person shapes, and zero copula. Case appears only through the forms
  the exchange needs. Every lesson is prerequisite-ordered and below five
  minutes, while the cross-language and etymological depth remains intact.

See [`roadmap.md`](./roadmap.md) for the plan toward B1 and
[`session-map.md`](./session-map.md) for how the lessons compose into commute
sessions.

---

## For contributors

Everything below this line is about how the track is built and checked. It is
here for people working on the curriculum; nothing in it is needed to learn the
language.

## What each chapter lets you do

[`chapters.json`](./chapters.json) is the HL05 capability ledger: per chapter, one
first-person can-do sentence and the lesson that pays it off.

- **Chapter 2** — *"I can give my name in Russian, ask for someone else's, and
  pick ты or вы to match how well I know them."* Payoff:
  [`RU-C02-kak-cross-language`](./lessons/RU-C02-kak-cross-language.md), a task —
  ask *Как вас зовут?* and account for its shape.

  Two honest caveats. Russian has no `core/book-generation.json` targets, so the
  chapter title and label come from
  [`book/chapters/ch02-introducing-yourself.tex`](./book/chapters/ch02-introducing-yourself.tex).
  And the payoff is not the chapter's `practice-mix` consolidation, because those
  three lessons are still schema v1 and declare no knowledge atoms; the last
  schema-v2 lesson by sequence stands in. Representativeness is therefore 3/15
  (0.20), well under the 0.5 floor — recorded rather than padded, and it closes
  when the practice lessons migrate.

**Chapter 1 is not in the ledger**, and that gap is deliberate: all twelve of its
lessons are schema v1, so it has no assessable payoff to point at. A placeholder
would hide debt the HL05 gap report is meant to surface.

## Read and practise

- [`book/book.tex`](./book/book.tex) builds the free two-chapter starter edition
  with XeLaTeX and the vendored Cyrillic font.
- Merged editions appear in the public human-languages book catalog.

## Status

Chapters 1 and 2 are authored as lessons and as a downloadable LaTeX starter
book typeset with the vendored `NotoSansCyrillic-Static.ttf`.
The six-lesson naming chain is also schema-v2 canonical content shared by the
app, with both mapped non-lexical Russian frontiers using objective activities.
