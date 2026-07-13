# Human Languages

A personal, etymology-driven curriculum for learning spoken languages,
delivered two ways from the same content: a LaTeX **book** per language
(meant for free publication, CC BY-SA 4.0) and ~5-minute **units** meant to
be consumed by ear during a daily car commute. Framework and rationale live
in [`HL00`](../../specs/HL00-human-language-curriculum-framework.md) — read
that first; this page is just an index.

Every track shares the same shape:

```
<language>/
  README.md          what this track is, how to use it, current progress
  CHANGELOG.md        per-chapter content additions
  roadmap.md          year-long Part/Chapter skeleton
  session-map.md      how units compose into commute sessions + review schedule
  units/*.md           the practice lesson files (new | review | practice-mix | morphology)
  book/                the LaTeX book (book.tex, chapters/*.tex)
```

## Tracks

| Language | Status |
|---|---|
| [Spanish](./spanish/README.md) | Pilot — Part 0 + Chapter 1 authored (units + book) |
| French | Planned (after Spanish proves the format) |
| German | Planned |
| Arabic | Planned |
| Hindi | Planned |
| Tamil | Planned |
| Kannada | Planned |
| Telugu | Planned |
| Malayalam | Planned |

Spanish is the pilot: the format, the spaced-repetition schedule, and the
etymology methodology all get proven out here before the same pattern is
replicated to the other eight languages. Once a second track exists, units
start sharing `concept_tag`s across languages for cross-language interleaving
(see `HL00`'s Interleaving section) — that's not populated yet.
