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
  curriculum.d/              ordered shared-spine path + extensions, sharded by entry
  chapters.d/                authored chapter can-dos and payoffs, one file per chapter
  roadmap.md                  themed-chapter skeleton
  session-map.md              how lessons compose into commute sessions + review schedule
  pronunciation-reference.md   the sounds, to look up on demand (never a gate)
  lessons/*.md                 deep one-word practice lessons (slug-named)
  book/                        the LaTeX book (book.tex, chapters/*.tex)
```

The machine-readable layer alongside the tracks is:

```text
core/languages.json             complete active-language registry and default mix order
core/spine.d/*.json             ordered, language-independent can-do spine: one file per node
core/book-generation.d/*.json  per-language generated-book declarations
core/latex-warning-baseline.json  per-track LaTeX warning debt the book gate holds the line on
core/lesson-modality/*.json     generated per-language voice/sight/pen and chapter prefixes
core/generated-book-hashes/<language>.d/ generated book hashes, one JSON owner per chapter
core/generated-narration-hashes/<language>.d/ generated narration hashes, one JSON owner per chapter
progress/*.md                   generated per-language progress cards for conflict-free authoring
core/figure-generation.json     configured canonical-data SVG figures and safe book targets
core/generated-figure-hashes.json generated: figure source/SVG drift fingerprints
concepts/taxonomy.json          cross-language semantic join keys
data/scripts/*.json             writing-system inventories and teaching metadata
```

The two generated-hash families use stable four-digit chapter owners rather
than per-language arrays: `_meta.json` carries document-wide fields and
`NNNN.json` carries exactly one chapter. There is no tracked language or corpus
aggregate. This lets independent agents regenerate different chapters of the
same language without sharing a manifest; `check:books` and `check:narration`
still fold and verify the complete corpus in memory. See
[`HL29`](../../specs/HL29-sharded-generated-chapter-hash-ownership.md).

### Sharded ledgers: `X.d/` (HL21)

A ledger that many people append to at once is a ledger that everybody conflicts
on. So the shared spine is stored as a DIRECTORY, one file per node:

```text
core/spine.d/_meta.json                    version, stages, strands
core/spine.d/0010-SPINE-MEET-GREET.json    one node
core/spine.d/0020-SPINE-COURTESY-THANK.json
```

Two people adding two nodes now write two different filenames, and git merges
them without noticing there was ever a question. The loader reads `X.d/` when it
exists and falls back to `X.json` when it does not, so ledgers migrate one at a
time.

The ordinal prefix is what makes sorted filename order reproduce AUTHORED order —
the spine is a ladder from pre-A1 to C2 and is not alphabetical. Ordinals are
spaced by ten so a node can be inserted as `0015` without renaming its
neighbours, which would be its own merge conflict.

`core/spine.json` no longer exists. Language Ladder's Vite plugin folds the
canonical shards into a virtual browser module at build time, with a key table
bounded by tracks rather than authored elements. The same is true for every
track's `curriculum.d/` and `chapters.d/`. Validate the shard-only corpus with:

```sh
npm run check:shards    # rebuilds in memory; fails if an aggregate returns
```

Edit only the owning shard. A resurrected monolith is ignored by readers and
therefore fails CI.

Generated Class-B figures live beside the book that consumes them under
`<language>/book/figures/`. The data package renders them from canonical lesson
claims, Language Ladder bundles the same SVG, and the unified books job verifies the
hash manifest before converting SVG to PDF for XeLaTeX. Run `npm run
generate:figures` or `npm run check:figures` in
`code/packages/typescript/human-language-data`; authored image paths are restricted
to relative `figures/*` targets and cannot escape a track.

Every registered track has one `curriculum.d/`. Its ordered path can revisit a
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

`core/lesson-modality/*.json` is generated, never authored. Each language owns one
independently mergeable shard recording for every lesson
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

### `book.tex` is generated (HL21 section 6)

`book.tex` was the last hand-maintained link in the lesson → chapter → book
chain, and the only one that could be forgotten without anything failing: a
chapter whose `\input` line is missing simply does not appear in the book, while
its `.tex` stays generated, committed and hash-checked. That has happened.

It is now split by **origin**, not by size:

```text
<track>/book/frontmatter.tex   AUTHORED — \documentclass, titlepage, licence,
                               preface, \tableofcontents, \mainmatter
<track>/book/backmatter.tex    AUTHORED — \backmatter, appendix inputs, \end{document}
<track>/book/book.tex          GENERATED — the two, with the derived \input list between
```

Edit the authored halves; never `book.tex` itself. `npm run generate:books`
rebuilds it and `npm run check:books` gates it, exactly like the chapters.

The chapter list comes from `core/book-generation.json` — **both** `targets[]`
and `handwritten[]`, merged by chapter number. Using `targets` alone silently
drops every hand-authored chapter.

Handwritten chapters also fail closed against canonical schema-v2 lessons. Each
such lesson must appear in exactly one manifest state:

- `embeddedLessonIds` means the lesson is learner-visible in the protected TeX
  body. The body must carry both `% canonical-insertion: <lesson-id>` and
  `\label{lesson:<lesson-id>}` evidence.
- `omittedLessonIds` is explicit migration debt and requires a positive
  `omissionIssue` pointing to its dependency-linked GitHub issue.

`npm run check:books` rejects undeclared lessons, stale or cross-chapter IDs,
duplicate states, issue-less omission debt, and embedded lessons without body
evidence. Narration, hashes, activities, answer keys, and source comments cannot
substitute for learner-visible book content. The pinned starting debt is 45
lessons across nine languages in #13117; that count may only move downward as
the handwritten bodies are repaired or fully generated.

Nothing checks that the LaTeX actually *compiles*; `check:books` only proves the
bytes match the generator. That gap is real — `src/book.ts`'s escape map was
once found missing a `ǵ`, which only a compiler catches. So:

```sh
npm run check:compile             # every track, ~100s
npm run check:compile spanish     # one track
```

It is opt-in and deliberately not part of `npm test`. Tracks with SVG figures
need `rsvg-convert` (or Inkscape, or ImageMagick's `magick`) on PATH, because
chapters reference figures as `.pdf` and only the `.svg` is committed; without a
converter those tracks are **skipped with a message**, not failed.

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

Each language owns an independently generated card under [`progress/`](./progress/).
Keeping those facts in one file per track lets Spanish, Gujarati, French, and every
other curriculum advance without rewriting this shared README. A release or docs
collector may assemble the cards into a consolidated table; authoring PRs never need
to wait for that presentation step.

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
