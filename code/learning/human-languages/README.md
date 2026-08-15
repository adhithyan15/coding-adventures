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
narration export a voice assistant reads aloud on a commute.
[`HL11`](../../specs/HL11-drizzled-script-ramp.md) covers the ramp none of the
others can express — the one climbed by a reader who does not already know the
alphabet: the book stays useful from page 1 while the script drizzles in one letter
at a time behind it, and no lesson may ask the reader to decode a glyph it has not
taught.
[`HL12`](../../specs/HL12-indic-pre-a1-to-c2.md) carries those six Indic tracks the
rest of the way, pre-A1 to C2, and turns on the observation that decoding and
meaning are two different ramps: the script one is finite and *ends*, the meaning
one is the whole climb. A lesson may sit at the frontier of one or the other,
never both, because a reader who fails a lesson that is new in both cannot tell
which one they failed. [`HL13`](../../specs/HL13-spine-layout-and-replication.md) sets the method for
everything above: the spine is laid out in **Spanish**, the script addendum in
**Tamil**, and both are then replicated across every language — because six
tracks in lockstep means every design mistake is made six times before anyone
finds it. This page is just an index.

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
core/figure-generation.json     configured canonical-data SVG figures and safe book targets
core/generated-figure-hashes.json generated: figure source/SVG drift fingerprints
concepts/taxonomy.json          cross-language semantic join keys
data/scripts/*.json             writing-system inventories and teaching metadata
```

Generated Class-B figures live beside the book that consumes them under
`<language>/book/figures/`. The data package renders them from canonical lesson
claims, Language Ladder bundles the same SVG, and the unified books job verifies the
hash manifest before converting SVG to PDF for XeLaTeX. Run `npm run
generate:figures` or `npm run check:figures` in
`code/packages/typescript/human-language-data`; authored image paths are restricted
to relative `figures/*` targets and cannot escape a track.

Every registered track has one `curriculum.json`. Its ordered path can revisit a
shared spine node, attaches required/supporting/reference extensions before,
inline with, or after a local segment, and explicitly records canonical concepts
that the track omits or deliberately teaches elsewhere. The data-package gate
proves that every registered map covers its schema-v2 and canonical lessons without
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
rather than written into every lesson frontmatter file, which would create one place
per lesson for it to go stale; the authored `modality:` override with a
`modality_reason:` remains available
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

The same canonical lessons power the public
[Language Ladder practice app](https://adhithyan15.github.io/coding-adventures/language-ladder/),
where any mix of tracks can advance independently through the shared spine and
enter cross-language review only after focused retrieval.

## Tracks

<!-- BEGIN GENERATED TRACK PROGRESS -->
| Language | Family / script | Canonical lessons | Mapped lessons | Book progress |
|---|---|---:|---:|---|
| [Spanish](./spanish/README.md) | Romance / Latin | 450 | 450 | 274 chapters; through Ch. 274; 274 generated |
| [Latin](./latin/README.md) | Italic / Latin | 88 | 88 | 43 chapters; through Ch. 43; 42 generated |
| [French](./french/README.md) | Romance / Latin | 105 | 91 | 31 chapters; through Ch. 31; 15 generated |
| [German](./german/README.md) | Germanic / Latin | 106 | 91 | 31 chapters; through Ch. 31; 15 generated |
| [Arabic](./arabic/README.md) | Semitic / Arabic | 100 | 98 | 36 chapters; through Ch. 36; 34 generated |
| [Hindi](./hindi/README.md) | Indo-Aryan / Devanagari | 145 | 139 | 41 chapters; through Ch. 41; 36 generated |
| [Tamil](./tamil/README.md) | Dravidian / Tamil | 169 | 165 | 41 chapters; through Ch. 41; 36 generated |
| [Kannada](./kannada/README.md) | Dravidian / Kannada | 125 | 121 | 42 chapters; through Ch. 42; 37 generated |
| [Telugu](./telugu/README.md) | Dravidian / Telugu | 124 | 120 | 42 chapters; through Ch. 42; 37 generated |
| [Malayalam](./malayalam/README.md) | Dravidian / Malayalam | 130 | 126 | 42 chapters; through Ch. 42; 37 generated |
| [Italian](./italian/README.md) | Romance / Latin | 88 | 87 | 25 chapters; through Ch. 25; 24 generated |
| [Portuguese](./portuguese/README.md) | Romance / Latin | 96 | 95 | 26 chapters; through Ch. 26; 25 generated |
| [Marathi](./marathi/README.md) | Indo-Aryan / Devanagari | 62 | 57 | 13 chapters; through Ch. 13; 8 generated |
| [Punjabi](./punjabi/README.md) | Indo-Aryan / Gurmukhi | 61 | 54 | 13 chapters; through Ch. 13; 8 generated |
| [Bengali](./bengali/README.md) | Indo-Aryan / Bengali | 70 | 66 | 15 chapters; through Ch. 15; 10 generated |
| [Gujarati](./gujarati/README.md) | Indo-Aryan / Gujarati | 59 | 55 | 12 chapters; through Ch. 12; 7 generated |
| [Russian](./russian/README.md) | Slavic / Cyrillic | 64 | 56 | 13 chapters; through Ch. 13; 11 generated |
| [Sanskrit](./sanskrit/README.md) | Indo-Aryan / Devanagari | 114 | 111 | 20 chapters; through Ch. 20; 15 generated |
| [Persian](./persian/README.md) | Iranian / Perso-Arabic | 59 | 59 | 14 chapters; through Ch. 14; 12 generated |
| [Urdu](./urdu/README.md) | Indo-Aryan / Urdu-Nastaliq | 59 | 59 | 15 chapters; through Ch. 15; 13 generated |
| [Mandarin Chinese](./chinese/README.md) | Sinitic / Chinese | 7 | 7 | 1 chapter; through Ch. 1; 1 generated |
| [Japanese](./japanese/README.md) | Japonic / Japanese | 8 | 8 | 1 chapter; through Ch. 1; 1 generated |
<!-- END GENERATED TRACK PROGRESS -->

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
