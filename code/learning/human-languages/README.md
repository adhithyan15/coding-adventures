# Human Languages

A personal, etymology-driven curriculum for learning spoken languages,
delivered two ways from the same content: a LaTeX **book** per language
(meant for free publication, CC BY-SA 4.0) and ~5-minute **units** meant to
be consumed by ear during a daily car commute. Framework and rationale live
in [`HL00`](../../specs/HL00-human-language-curriculum-framework.md) — read
that first. The migration to an ordered shared spine, strict sub-five-minute
lessons, Persian and Urdu extensions, and a single book/app/practice content
pipeline is specified in
[`HL04`](../../specs/HL04-shared-spine-and-content-pipeline.md). This page is
just an index.

Every track shares the same shape:

```text
<language>/
  README.md                  what this track is, how to use it, current progress
  CHANGELOG.md                per-chapter content additions
  roadmap.md                  themed-chapter skeleton
  session-map.md              how lessons compose into commute sessions + review schedule
  pronunciation-reference.md   the sounds, to look up on demand (never a gate)
  lessons/*.md                 deep one-word practice lessons (slug-named)
  book/                        the LaTeX book (book.tex, chapters/*.tex)
```

The machine-readable layer alongside the tracks is:

```text
core/languages.json             complete active-language registry and default mix order
core/spine.json                 ordered, language-independent can-do spine
concepts/taxonomy.json          cross-language semantic join keys
data/scripts/*.json             writing-system inventories and teaching metadata
```

The data package also loads every existing `book/book.tex` and `book/chapters/ch*.tex`
losslessly and checks that each authored book chapter maps to its short Markdown
lessons. This preserves the well-received book narrative while the pipeline moves
toward generating book and app views from one lesson AST.

## Download the books

The [public book catalog](https://adhithyan15.github.io/coding-adventures/human-languages/books/)
offers every currently authored LaTeX book as a free PDF download. Pull requests
install TeX once, compile every book in one job, and retain the complete publication
bundle as a workflow artifact for review. After a change reaches `main`, the same
validated PDFs and a machine-readable `catalog.json` are published to GitHub Pages
automatically. Tracks without a `book/` directory are omitted until their long-form
edition is authored.

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
| [Marathi](./marathi/README.md) | Indo-Aryan / Devanagari | Chapters 1-5 authored (lessons + book) |
| [Punjabi](./punjabi/README.md) | Indo-Aryan / Gurmukhi (vendored font) | Chapter 1 (Greetings) authored (lessons + book) |
| [Bengali](./bengali/README.md) | Indo-Aryan / Bengali (vendored font) | Chapter 1 (Greetings) authored (lessons + book) |
| [Gujarati](./gujarati/README.md) | Indo-Aryan / Gujarati (vendored font) | Chapters 1-5 authored (new track, script inline) |
| [Tamil](./tamil/README.md) | Dravidian / Tamil (vendored font) | Chapters 1-2 authored (lessons + book) |
| [Kannada](./kannada/README.md) | Dravidian / Kannada (vendored font) | Chapters 1-2 authored (lessons + book) |
| [Telugu](./telugu/README.md) | Dravidian / Telugu (vendored font) | Chapters 1-2 authored (lessons + book) |
| [Malayalam](./malayalam/README.md) | Dravidian / Malayalam (vendored font) | Chapters 1-2 authored (lessons + book) |
| [Latin](./latin/README.md) | Italic / Latin (**taproot**) | Chapter 1 (Greetings) authored (lessons + book) |
| [Sanskrit](./sanskrit/README.md) | Indo-Aryan / Devanagari (**taproot**) | Chapter 1 (Greetings) authored (lessons + book) |
| [Russian](./russian/README.md) | Slavic / Cyrillic | Starter curriculum authored |
| [Persian](./persian/README.md) | Iranian / Perso-Arabic | Shared-spine pilot: greetings, responses, and self-introduction |
| [Urdu](./urdu/README.md) | Indo-Aryan / Urdu Nastaliq | Shared-spine pilot: greetings, responses, and self-introduction |

Spanish is the pilot that proved the format; every other track replicates it,
grounding each word against English plus whatever languages the learner already
knows (see `HL00`). Non-Latin scripts are taught **inline** — letters introduced
inside the word that first needs them, never as a gated reading course — using
vendored static Noto fonts (see [`_fonts/`](./_fonts/)) so local and CI builds
render identically.

**Latin and Sanskrit are taproot tracks.** Rather than being learned for
conversation, they are the classical sources the other tracks keep pointing back
to: Latin is the parent of the Spanish/French/Italian/Portuguese greetings (and
half of English's vocabulary), and Sanskrit is the parent of the
Hindi/Marathi/Punjabi/Bengali greetings — while both, as Indo-European sisters,
also reach into English (*na* ↔ *no* ↔ *nōn*, *su-* ↔ Greek *eu-*, √gam ↔ *come*).
They tie the two halves of the curriculum together.
