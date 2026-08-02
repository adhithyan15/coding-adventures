# Russian

A track of the [Human Languages](../README.md) curriculum — the first on the
**Cyrillic** script — built on the same
[`HL00`](../../../specs/HL00-human-language-curriculum-framework.md) framework:
one word per lesson, stable slug ids, etymology-first, the script taught inline,
grammar introduced only when a word needs it.

## What's different about the Russian track

- **Cyrillic taught inside the words — no reading course.** Each lesson has a
  *"The letters in this word"* section introducing exactly the letters that word
  needs. The track's spine is the **four false friends** — в=v, р=r, с=s, н=n —
  the letters that look Latin and lie; by the end of Chapter 1 they're fixed.
  Full per-letter decomposition lives in
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
  я → ты / вы → меня зовут… → как вас зовут? → очень приятно, plus cumulative
  formal and informal practice. Case appears only through the forms the exchange
  needs.

See [`roadmap.md`](./roadmap.md) for the plan toward B1 and
[`session-map.md`](./session-map.md) for how the lessons compose into commute
sessions.

## Read and practise

- [`book/book.tex`](./book/book.tex) builds the free two-chapter starter edition
  with XeLaTeX and the vendored Cyrillic font.
- Merged editions appear in the public human-languages book catalog.

## Status

Chapters 1 and 2 are authored as lessons and as a downloadable LaTeX starter
book typeset with the vendored `NotoSansCyrillic-Static.ttf`.
