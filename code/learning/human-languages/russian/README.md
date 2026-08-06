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
- **Drivable by ear.** 22 of the track's 28 lessons need only your ears; the
  other 6 are the five handwriting lessons and one cover-the-column retrieval
  drill. Russian used to be the least drivable track here, entirely because its
  cross-language comparisons were set as tables rather than said as sentences.
  They are sentences now, and Chapter 3 was written that way from the start —
  it is the track's first chapter that is drivable end to end.

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
- **Chapter 3 — Six verbs, and the one you never say**
  ([`lessons/RU-C03-*`](./lessons/)): быть (to be) → жить (to live) → знать (to
  know) → говорить (to speak) → видеть (to see) → идти (to go). One verb per
  lesson and one grammatical idea per verb — the **zero copula**, the **-у** that
  by itself means "I", **не** as the whole of English *don't*, the
  **-ешь / -ишь** families, the **д → ж** swap in *вижу*, and verbs of motion
  (*иду* now, against *хожу* habitually). The etymology carries the chapter:
  *быть* is **be**, *знать* is **know**, *видеть* is **wit** and **wise**, *жить*
  is **quick** in its older sense of *alive*, and *идти → шёл* is *go → went* in
  a second language. One new letter (**г**), and English *govern* flagged as the
  false friend it is.

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
  [`RU-C02-practice`](./lessons/RU-C02-practice.md), a dialogue — run the whole
  introduction with a stranger, then switch it for one friend by changing only
  the greeting and *вас → тебя*. It assesses ten of the chapter's fifteen
  introduced atoms, for a representativeness of **0.67** against the 0.5 floor.

  One honest caveat remains: Russian has no `core/book-generation.json` targets,
  so the chapter title and label come from
  [`book/chapters/ch02-introducing-yourself.tex`](./book/chapters/ch02-introducing-yourself.tex).

**Chapter 1 is not in the ledger**, and that gap is deliberate: all twelve of its
lessons are schema v1, so it has no assessable payoff to point at. A placeholder
would hide debt the HL05 gap report is meant to surface.

**Chapter 3 is not in the ledger either**, for a different and equally recorded
reason: the ledger is checked against the chapters the *book* has, and Russian's
book chapters are handwritten `.tex` files. No `ch03` was authored, so there is
no book chapter for a capability entry to describe. The six lessons are
canonical schema-v2 content and the app can serve them today; the book is what
is behind.

## Read and practise

- [`book/book.tex`](./book/book.tex) builds the free two-chapter starter edition
  with XeLaTeX and the vendored Cyrillic font.
- Merged editions appear in the public human-languages book catalog.

## Status

Chapters 1 and 2 are authored as lessons and as a downloadable LaTeX starter
book typeset with the vendored `NotoSansCyrillic-Static.ttf`.
The six-lesson naming chain is also schema-v2 canonical content shared by the
app, with both mapped non-lexical Russian frontiers using objective activities.
Chapter 3 is authored as six schema-v2 lessons only — no book chapter yet. It is
also the corpus's first realization of the shared `VERB-*` concepts (six of the
core forty), and the first content anywhere to sit on an A2 spine node.
