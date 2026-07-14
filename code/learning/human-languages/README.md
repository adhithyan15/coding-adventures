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

| Language | Family / script | Status |
|---|---|---|
| [Spanish](./spanish/README.md) | Romance / Latin | Pilot — Chapters 1-3 authored (lessons + book); ~30 lessons |
| [French](./french/README.md) | Romance / Latin | Chapters 1-2 authored (lessons + book) |
| [German](./german/README.md) | Germanic / Latin | Chapters 1-2 authored (lessons + book) |
| [Italian](./italian/README.md) | Romance / Latin | Chapter 1 (Greetings) authored (lessons + book) |
| [Portuguese](./portuguese/README.md) | Romance / Latin | Chapter 1 (Greetings) authored (lessons + book) |
| [Arabic](./arabic/README.md) | Semitic / Arabic (vendored font) | Chapters 1-2 authored (script inline) |
| [Hindi](./hindi/README.md) | Indo-Aryan / Devanagari | Chapters 1-2 authored (lessons + book) |
| [Marathi](./marathi/README.md) | Indo-Aryan / Devanagari | Chapter 1 (Greetings) authored (lessons + book) |
| [Punjabi](./punjabi/README.md) | Indo-Aryan / Gurmukhi (vendored font) | Chapter 1 (Greetings) authored (lessons + book) |
| [Bengali](./bengali/README.md) | Indo-Aryan / Bengali (vendored font) | Chapter 1 (Greetings) authored (lessons + book) |
| [Tamil](./tamil/README.md) | Dravidian / Tamil (vendored font) | Chapters 1-2 authored (lessons + book) |
| [Kannada](./kannada/README.md) | Dravidian / Kannada (vendored font) | Chapters 1-2 authored (lessons + book) |
| [Telugu](./telugu/README.md) | Dravidian / Telugu (vendored font) | Chapters 1-2 authored (lessons + book) |
| [Malayalam](./malayalam/README.md) | Dravidian / Malayalam (vendored font) | Chapters 1-2 authored (lessons + book) |

Spanish is the pilot that proved the format; every other track replicates it,
grounding each word against English plus whatever languages the learner already
knows (see `HL00`). Non-Latin scripts are taught **inline** — letters introduced
inside the word that first needs them, never as a gated reading course — using
vendored static Noto fonts (see [`_fonts/`](./_fonts/)) so local and CI builds
render identically.
