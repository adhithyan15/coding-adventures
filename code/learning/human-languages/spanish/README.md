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

## Five things this track does deliberately

1. **Etymology-driven.** Every new word traces to its root — mostly Latin,
   sometimes Arabic — and forward to an English cognate.
2. **A real book.** `book/` is a LaTeX volume, licensed CC BY-SA 4.0, that
   grows one chapter at a time as weeks get authored — see "The book" below.
3. **Word-formation as its own thread.** `morphology`-type units teach one
   Latin root at a time (e.g. *clamare* → *llamar/claim/exclaim*) — lexical
   Latin, riding alongside the Spanish content.
4. **A slow alphabet/sound-system introduction.** Part 0 (`units/ES-P0-U00*`)
   covers Spanish's sounds and stress rules before any vocabulary — short
   here since Spanish reuses the Latin alphabet, but the same pattern will
   be much larger for Arabic/Hindi/Tamil/Kannada/Telugu/Malayalam later.
5. **Grammar explained from zero, contrasted with English.** Every
   grammar-introducing unit has a Grammar Lens section: what the concept
   *is*, how English handles the same function (if it does), what's
   different — no assumed grammar vocabulary, matching how the curriculum's
   author actually learned English (immersion, not formal instruction).
   Grammatical gender gets this treatment as a standing thread, tagged on
   every noun from the first one onward.

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

- **Part 0 — Sounds & Letters**: Chapter 0 fully authored (`units/ES-P0-U00*`).
- **Part I — Foundations**: Chapter 1 fully authored (`units/`, sessions
  1-10) plus one morphology unit (`ES-P0-M01`). Chapters 2-4 not yet
  written.
- **Parts II-V**: skeleton only, in [`roadmap.md`](./roadmap.md).
- **Book**: Part 0 + Chapter 1 written in `book/`; grows chapter by chapter
  alongside `units/`.

See [`CHANGELOG.md`](./CHANGELOG.md) for what's been added, chapter by
chapter.

## Files

- [`roadmap.md`](./roadmap.md) — the full year, part by part, chapter by
  chapter.
- [`session-map.md`](./session-map.md) — which units make up which session,
  and the worked spaced-repetition schedule so far.
- [`units/`](./units/) — the practice lesson files.
- [`book/`](./book/) — the LaTeX book.
