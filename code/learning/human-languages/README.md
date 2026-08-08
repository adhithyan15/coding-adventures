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
[`HL07`](../../specs/HL07-spine-expansion-to-b1.md).
[`HL08`](../../specs/HL08-modality-gentle-ramp-and-the-drivable-course.md) marks which
chapters need eyes or a pen and which can be learned entirely by ear, and defines the
narration export a voice assistant reads aloud on a commute. This page is just an
index.

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
core/latex-warning-baseline.json  per-track LaTeX warning debt the book gate holds the line on
core/lesson-modality.json       generated: per-lesson voice/sight/pen and per-chapter drivable prefix
concepts/taxonomy.json          cross-language semantic join keys
data/scripts/*.json             writing-system inventories and teaching metadata
```

Every registered track has one `curriculum.json`. Its ordered path can revisit a
shared spine node, attaches required/supporting/reference extensions before,
inline with, or after a local segment, and explicitly records canonical concepts
that the track omits or deliberately teaches elsewhere. The data-package gate
proves that all 21 maps cover their schema-v2 and canonical lessons without
jumping over a prerequisite. Books and the app still read the lesson Markdown;
the map is the shared scheduling contract, not a second copy of the content.

The unified publication job also emits `curriculum-gaps.json` and
`curriculum-gaps.txt` beside the books. They record the effective duration budget
for every over-limit lesson, prerequisite omissions, lesson-to-book chapter
coverage, and each track's schema-migration status. Existing migration debt is
reported rather than hidden or treated as a new regression.

The same job also scans every compiled `book.log` and emits `latex-warnings.json`
beside the books. Overfull and underfull boxes, missing glyphs, hyperref warnings,
duplicate PDF destinations, and font substitutions are counted per track and compared
against `core/latex-warning-baseline.json`; a track fails only when it exceeds its
recorded numbers, and a track recorded as `null` has not been measured yet, so it is
reported and never failed. Until this gate existed, every track's "builds with zero
warnings" claim was prose that nothing checked.

`core/lesson-modality.json` is generated, never authored. It records for every lesson
whether it needs `voice`, `sight`, or a `pen`, and for every chapter how many of its
lessons a commuter can do in the car before hitting the first that needs eyes. This is
what lets **two editions build from one source**: the complete book keeps everything,
including the handwriting instruction, while the planned dictation-friendly driving
edition filters on `drivable` and keeps only what a driver can actually do. The same
job that builds the books runs `check:modality`, so the manifest cannot drift away from
the lessons it describes — a lesson that silently gained a paradigm table would
otherwise still advertise itself as safe to learn at 70mph. Modality stays derived
rather than written into 1,096 frontmatter files, which would be 1,096 places for it to
go stale; the authored `modality:` override with a `modality_reason:` remains available
for the genuinely exceptional lesson.

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
| [Spanish](./spanish/README.md) | Romance / Latin | Reference track — Chapters 1–39 authored; 169 lessons; 36 of the 40 core verbs — and the tranche that leaves **none** unrealized corpus-wide |
| [French](./french/README.md) | Romance / Latin | Chapters 1–27 authored; 89 lessons; 22 of the 40 core verbs |
| [German](./german/README.md) | Germanic / Latin | Chapters 1–27 authored; 92 lessons; 22 of the 40 core verbs |
| [Italian](./italian/README.md) | Romance / Latin | Chapters 1–21 authored; Chapters 2–21 canonical/generated for app + book; 73 lessons; 21 of the 40 core verbs |
| [Portuguese](./portuguese/README.md) | Romance / Latin | Chapters 1–22 authored; Chapters 2–22 canonical/generated for app + book; 22 of the 40 core verbs |
| [Arabic](./arabic/README.md) | Semitic / Arabic (vendored font) | Chapters 1–30 authored; Chapters 3–30 canonical/generated for app + book; first track to realize core `VERB-*` concepts and the first to reach A2 |
| [Hindi](./hindi/README.md) | Indo-Aryan / Devanagari | Chapters 1–33 authored; Chapters 6–33 canonical/generated for app + book; 11 inline writing steps |
| [Marathi](./marathi/README.md) | Indo-Aryan / Devanagari | Chapters 1–9 authored (lessons + book; Chapters 6–9 canonical/generated; Ch. 7–9 = 14 core verbs, 98% drivable) |
| [Punjabi](./punjabi/README.md) | Indo-Aryan / Gurmukhi (vendored font) | Chapters 1-6 authored (lessons + book, script inline) |
| [Bengali](./bengali/README.md) | Indo-Aryan / Bengali (vendored font) | Chapters 1–9 authored (lessons + book; Chapters 6–9 canonical/generated; Ch. 7–9 = 14 core verbs, 98% drivable) |
| [Gujarati](./gujarati/README.md) | Indo-Aryan / Gujarati (vendored font) | Chapters 1-6 authored (lessons + book, script inline) |
| [Tamil](./tamil/README.md) | Dravidian / Tamil (vendored font) | Chapters 1–31 authored; Chapters 6–31 canonical/generated for app + book; 8 inline writing steps |
| [Kannada](./kannada/README.md) | Dravidian / Kannada (vendored font) | Chapters 1-2 authored (lessons + book) |
| [Telugu](./telugu/README.md) | Dravidian / Telugu (vendored font) | Chapters 1-2 authored (lessons + book) |
| [Malayalam](./malayalam/README.md) | Dravidian / Malayalam (vendored font) | Chapters 1–31 authored; Chapters 6–31 canonical/generated for app + book |
| [Latin](./latin/README.md) | Italic / Latin (**taproot**) | Chapters 1–43 authored; 88 lessons; 31 of the 40 core verbs |
| [Sanskrit](./sanskrit/README.md) | Indo-Aryan / Devanagari (**taproot**) | Chapters 1–9 authored (lessons + book; Chapters 6–9 canonical/generated) |
| [Russian](./russian/README.md) | Slavic / Cyrillic | Chapters 1–5 authored (lessons + book; Chapters 3–5 canonical/generated). Chapters 4–5 are the eight core verbs; aspect is named, not finished |
| [Persian](./persian/README.md) | Iranian / Perso-Arabic | Eight authored shared-spine chapters; 33 canonical lessons, including thirteen core verbs |
| [Urdu](./urdu/README.md) | Indo-Aryan / Urdu Nastaliq | Five authored shared-spine chapters; 20 canonical lessons |
| [Mandarin Chinese](./chinese/README.md) | Sinitic / Chinese (vendored font subset) | Chapter 1 authored; 7 canonical lessons, book chapter generated. **Scale test** — see that README for what the method does and does not carry outside Indo-European |
| [Japanese](./japanese/README.md) | Japonic / hiragana + katakana + kanji (vendored font) | Chapter 1 authored; 8 canonical lessons, chapter generated for app + book |

Spanish is the pilot that proved the format; every other track replicates it,
grounding each word against English plus whatever languages the learner already
knows (see `HL00`). Non-Latin scripts are taught **inline** — letters introduced
inside the word that first needs them, never as a gated reading course — using
vendored static Noto fonts (see [`_fonts/`](./_fonts/)) so local and CI builds
render identically.

**Mandarin is the scale test.** The first twenty tracks are Indo-European or
Dravidian, and the method's engine — anchoring each new word to English words the
reader already owns through a shared ancestor — depends on that shared ancestry.
Chinese has none, is logographic rather than alphabetic, and is tonal. Its track
was added to find out which parts of the framework describe language in general
and which quietly described Indo-European, and
[`chinese/README.md`](./chinese/README.md) states the answers plainly, including
the one place the signature device does not transfer at all.

**Latin and Sanskrit are taproot tracks.** Rather than being learned for
conversation, they are the classical sources the other tracks keep pointing back
to: Latin is the parent of the Spanish/French/Italian/Portuguese greetings (and
half of English's vocabulary), and Sanskrit is the parent of the
Hindi/Marathi/Punjabi/Bengali greetings — while both, as Indo-European sisters,
also reach into English (*na* ↔ *no* ↔ *nōn*, *su-* ↔ Greek *eu-*, √gam ↔ *come*).
They tie the two halves of the curriculum together.

**Japanese is the first track with no taproot the reader already owns.** Every
other track here is Indo-European or Semitic and can ground a new word in Latin,
Sanskrit, or a Semitic root the reader meets through English. Japanese cannot,
and no connection is invented for it. The method is redirected instead, onto
three things that are real: the **Sino-Japanese** layer (日本語 has checkable
relatives in Mandarin and Korean), **internal** etymology (ありがとう ← 有り難し,
"hard to exist"), and genuine **shared borrowings** (コーヒー and English *coffee*
both from Arabic *qahwa*, by different roads). Where no honest connection exists,
the lesson says so. Japanese is also the first track that needs three writing
systems at once; see [`japanese/README.md`](./japanese/README.md) for what that
cost the schema.
