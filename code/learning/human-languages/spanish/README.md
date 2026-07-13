# Spanish

The pilot track for the [Human Languages](../README.md) curriculum. Goal:
absolute-beginner to B1 ("can hold a normal day-to-day conversation") over a
year, delivered two ways from the same underlying content — a **book**
(`book/`, LaTeX, meant for free publication) and **practice units**
(`units/`, ~5-minute pieces consumed during a daily car commute). Framework
details (unit anatomy, spaced-repetition schedule, etymology/gender
methodology) are in
[`HL00`](../../../specs/HL00-human-language-curriculum-framework.md) — this
README is "how to actually use this."

## What this track does deliberately

1. **One word per lesson, gone deep.** Not "the ten greetings" — *hola* is a
   lesson, *buenos días* is a lesson. A few minutes each. See `lessons/`.
2. **The widest honest web of English cousins.** Every word reaches for all
   the English relatives it can *truthfully* claim (e.g. *quiero* ← *quaerere*
   → query, inquire, require, acquire, conquer, exquisite) — because the more
   live connections a word has, the more places it sticks. False cousins are
   flagged, not smuggled in.
3. **The reason, not the rule.** Why *buenos días*, plural? Because it's the
   fossil of a blessing ("may God give you good days"). Each lesson digs out
   the cultural/idiomatic *why*, the part Spanish 101 skips.
4. **Prefixes and suffixes taught in context.** When a root builds its family
   by prefix (*in-* + *-quiry* → *inquiry*), the construction is shown and
   named — a skill that pays off across the learner's whole English
   vocabulary.
5. **Pronunciation inline, never a gate.** No alphabet chapter to sit through.
   Each lesson names the sounds *its* word needs; the full system lives in
   [`pronunciation-reference.md`](./pronunciation-reference.md) to look up on
   demand.
6. **Grammar from zero, contrasted with English.** Grammar concepts are
   introduced in context on the first word that needs them, explained with no
   assumed terminology.

## How to use this in the car

1. Before you drive, open [`session-map.md`](./session-map.md) and find the
   next session you haven't done yet.
2. Read (or have read to you) the units listed for that session, in order —
   due reviews first, then the new unit, then any morphology/practice-mix
   unit. Each unit is self-paced: pause, speak your answer out loud, then
   continue.
3. That's the core block, ~15-25 minutes. Longer drive? Keep going into the
   bonus queue — extra review units, never anything you haven't earned yet.
4. Next drive, start from the top of the next session. Don't skip ahead —
   the review schedule assumes you did the units in order.

Units are plain Markdown, written in an audio-script style (`[PAUSE]`,
`[REPEAT x2]`, `[YOU SAY: ...]`) so they can be read aloud by you, a
passenger, or (eventually) a voice pipeline — that pipeline doesn't exist
yet; see `HL00`'s "Explicitly Out of Scope" section.

## The book

`book/` is a LaTeX book, compiled with XeLaTeX (`fontspec` + `polyglossia`
— required later for Arabic/Devanagari/Dravidian scripts, so it's the
standard from day one even though Spanish itself doesn't need it).
Licensed CC BY-SA 4.0. To build it locally:

```
cd book && latexmk -xelatex book.tex   # or: ./build.sh / .\build.ps1
```

Requires a LaTeX distribution with `xelatex`/`latexmk` on PATH (MiKTeX or
TeX Live). The compiled PDF isn't committed — it's regenerated from source,
same as build artifacts elsewhere in this repo.

The book and `units/` are two views of the same content, not independently
maintained — the book is the continuous read, the units are the same
material sliced into car-sized practice pieces.

## Progress

The track was **redesigned** after early feedback: from coarse units (ten
greetings crammed together, a front-loaded pronunciation chapter) to deep
one-word lessons with inline sounds and a full cousin web. Current state:

- **Chapter 1 — First Words**: 10 deep lessons in [`lessons/`](./lessons/)
  (`ES-C01-L01`–`L10`): hola, buenos días, buenas tardes, buenas noches,
  gracias, por favor, adiós, ¿cómo estás?, me llamo, quiero. Fully authored
  and in the book.
- **Pronunciation**: [`pronunciation-reference.md`](./pronunciation-reference.md)
  (look-up reference, not a chapter).
- **Later chapters**: skeleton in [`roadmap.md`](./roadmap.md).
- **`units/` (legacy)**: the pre-redesign coarse-grained content (old Part 0
  + Part I). Kept as source material to be re-excavated into deep lessons;
  **superseded**, not the current model. Being migrated `units/` → `lessons/`.

See [`CHANGELOG.md`](./CHANGELOG.md) for the full history including the
redesign.

## Files

- [`lessons/`](./lessons/) — the deep one-word practice lessons (current model).
- [`pronunciation-reference.md`](./pronunciation-reference.md) — sounds, to
  look up on demand.
- [`roadmap.md`](./roadmap.md) — the chapter skeleton.
- [`session-map.md`](./session-map.md) — which lessons make up which session.
- [`book/`](./book/) — the LaTeX book.
- [`units/`](./units/) — **legacy**, pre-redesign; being migrated to `lessons/`.
