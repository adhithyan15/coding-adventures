# Human Languages

A personal, etymology-driven curriculum for learning spoken languages,
delivered two ways from the same content: a LaTeX **book** per language
(meant for free publication, CC BY-SA 4.0) and ~5-minute **units** meant to
be consumed by ear during a daily car commute. Framework and rationale live
in [`HL00`](../../specs/HL00-human-language-curriculum-framework.md) — read
that first. The migration to an ordered shared spine, strict sub-five-minute
lessons, Persian and Urdu extensions, and a single book/app/practice content
pipeline is specified in
[`HL04`](../../specs/HL04-shared-spine-and-content-pipeline.md). The move to
chapter-sized deployable capability — every chapter promising something the reader can
use immediately — is specified in
[`HL05`](../../specs/HL05-chapter-capability-and-step-by-step-shape.md), with the
book's visual system and inline script-writing figures in
[`HL06`](../../specs/HL06-visual-system.md) and the spine's growth through B1 in
[`HL07`](../../specs/HL07-spine-expansion-to-b1.md). This page is just an index.

Every track shares the same shape:

```text
<language>/
  README.md                  what this track is, how to use it, current progress
  CHANGELOG.md                per-chapter content additions
  curriculum.json            ordered shared-spine realization path + local extensions
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

Every registered track has one `curriculum.json`. Its ordered path can revisit a
shared spine node, attaches required/supporting/reference extensions before,
inline with, or after a local segment, and explicitly records canonical concepts
that the track omits or deliberately teaches elsewhere. The data-package gate
proves that all 20 maps cover their schema-v2 and canonical lessons without
jumping over a prerequisite. Books and the app still read the lesson Markdown;
the map is the shared scheduling contract, not a second copy of the content.

The unified publication job also emits `curriculum-gaps.json` and
`curriculum-gaps.txt` beside the books. They record the effective duration budget
for every over-limit lesson, prerequisite omissions, lesson-to-book chapter
coverage, and each track's schema-migration status. Existing migration debt is
reported rather than hidden or treated as a new regression.

The data package also loads every existing `book/book.tex` and `book/chapters/ch*.tex`
losslessly and checks that each authored book chapter maps to its short Markdown
lessons. This preserves the well-received book narrative while the pipeline moves
toward generating book and app views from one lesson AST.

## Download the books

The [public book catalog](https://adhithyan15.github.io/coding-adventures/human-languages/books/)
offers every currently authored LaTeX book as a free PDF download. Pull requests
install one focused XeLaTeX toolchain, compile every book in one job, and retain the
complete publication bundle as a workflow artifact for review. A dependency
preflight verifies the engine, every package used by the books, RTL support, and
Latin Modern before compilation begins. After a change reaches `main`, the same
validated PDFs and a machine-readable `catalog.json` are published to GitHub Pages
automatically. Tracks without a `book/` directory are omitted until their long-form
edition is authored.

## Tracks

| Language | Family / script | Status |
|---|---|---|
| [Spanish](./spanish/README.md) | Romance / Latin | Pilot — Chapters 1-3 authored (lessons + book); ~30 lessons |
| [French](./french/README.md) | Romance / Latin | Chapters 1-2 authored (lessons + book) |
| [German](./german/README.md) | Germanic / Latin | Chapters 1-2 authored (lessons + book) |
| [Italian](./italian/README.md) | Romance / Latin | Chapters 1–17 authored; Chapters 2–17 canonical/generated for app + book |
| [Portuguese](./portuguese/README.md) | Romance / Latin | Chapter 1 (Greetings) authored (lessons + book) |
| [Arabic](./arabic/README.md) | Semitic / Arabic (vendored font) | Chapters 1–27 authored; Chapters 3–27 canonical/generated for app + book |
| [Hindi](./hindi/README.md) | Indo-Aryan / Devanagari | Chapters 1–33 authored; Chapters 6–33 canonical/generated for app + book; 11 inline writing steps |
| [Marathi](./marathi/README.md) | Indo-Aryan / Devanagari | Chapters 1-6 authored (lessons + book) |
| [Punjabi](./punjabi/README.md) | Indo-Aryan / Gurmukhi (vendored font) | Chapters 1-6 authored (lessons + book, script inline) |
| [Bengali](./bengali/README.md) | Indo-Aryan / Bengali (vendored font) | Chapters 1–6 authored (lessons + book; Chapter 6 canonical/generated) |
| [Gujarati](./gujarati/README.md) | Indo-Aryan / Gujarati (vendored font) | Chapters 1-6 authored (lessons + book, script inline) |
| [Tamil](./tamil/README.md) | Dravidian / Tamil (vendored font) | Chapters 1–31 authored; Chapters 6–31 canonical/generated for app + book; 8 inline writing steps |
| [Kannada](./kannada/README.md) | Dravidian / Kannada (vendored font) | Chapters 1-2 authored (lessons + book) |
| [Telugu](./telugu/README.md) | Dravidian / Telugu (vendored font) | Chapters 1-2 authored (lessons + book) |
| [Malayalam](./malayalam/README.md) | Dravidian / Malayalam (vendored font) | Chapters 1–31 authored; Chapters 6–31 canonical/generated for app + book |
| [Latin](./latin/README.md) | Italic / Latin (**taproot**) | Chapters 1–36 authored; Chapters 2–36 canonical/generated for app + book |
| [Sanskrit](./sanskrit/README.md) | Indo-Aryan / Devanagari (**taproot**) | Chapters 1–6 authored (lessons + book; Chapter 6 canonical/generated) |
| [Russian](./russian/README.md) | Slavic / Cyrillic | Chapters 1-2 authored (lessons + book) |
| [Persian](./persian/README.md) | Iranian / Perso-Arabic | Five authored shared-spine chapters; 20 canonical lessons |
| [Urdu](./urdu/README.md) | Indo-Aryan / Urdu Nastaliq | Five authored shared-spine chapters; 20 canonical lessons |

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
