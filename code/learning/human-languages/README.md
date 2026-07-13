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
  README.md                  what this track is, how to use it, current progress
  CHANGELOG.md                per-chapter content additions
  roadmap.md                  themed-chapter skeleton
  session-map.md              how lessons compose into commute sessions + review schedule
  pronunciation-reference.md   the sounds, to look up on demand (never a gate)
  lessons/*.md                 deep one-word practice lessons (slug-named)
  book/                        the LaTeX book (book.tex, chapters/*.tex)
```

## Tracks

| Language | Status |
|---|---|
| [Spanish](./spanish/README.md) | Pilot — Chapters 1-3 authored (lessons + book); ~30 lessons |
| [French](./french/README.md) | **Chapter 1 (Greetings) authored** (lessons + book) |
| German | Planned (next — Germanic roots + shared-with-English cognates) |
| Arabic | Planned (learner reads it, rusty; script inline) |
| Hindi | Planned (learner reads Devanagari; Sanskrit + Persian/Arabic roots) |
| Tamil | Planned (native speaker; formal grammar + native/Sanskrit doublets) |
| Kannada | Planned (new script) |
| Telugu | Planned (new script) |
| Malayalam | Planned (new script) |

Spanish is the pilot that proved the format; French is the first replication,
and it grounds each word against English **and Spanish** (the learner's
in-progress language), foregrounding the Romance twins' differences. The other
tracks follow in the order above, each grounded on English plus whatever
languages the learner already knows by then (see `HL00`).
