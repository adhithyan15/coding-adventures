# HL06 — Visual System: Figures, Script Diagrams, and Illustrations

## Status and purpose

This spec gives the Human Languages books a visual layer: stroke-order diagrams that
teach the reader to *write* a script, data-derived diagrams that make etymology and
sound visible, and illustrations that make the books pleasant to read.

It extends [HL00](./HL00-human-language-curriculum-framework.md)'s "Book Format" and
[HL04](./HL04-shared-spine-and-content-pipeline.md)'s one-source pipeline. Figures are
a **fourth output view** of the canonical content, not a parallel asset library.

The design requirements are:

1. A reader learning a non-Latin script must see how the pen actually moves.
2. Book and app must teach handwriting from **one** source, not two.
3. Every figure that carries a factual claim must be derived from data and verifiable
   in CI.
4. Decorative art may never carry a factual claim.
5. The books stay reproducible: a clean checkout plus CI must rebuild every PDF.

## The gap this closes

| Observation | Value |
|---|---|
| Images in any of the 20 books | **0** |
| Graphics packages loaded in any preamble | **0** |
| Renderer support for `![alt](src)` | none — silently degrades to `!\href{src}{alt}` |
| Authored geometric stroke paths (`DUCTUS`) | **1 letter** (Tamil ம) |
| UI or book consumers of that stroke data | **0** |
| Scripts with cited prose stroke order | 9 scripts, 190 letters, rendered as a plain `<ol>` |

The most striking of these is the fourth. `strokes.ts` contains a complete, carefully
reasoned pen-path model — strokes as pen-down runs, segments as labelled parts that
must meet head-to-tail, `penPathD()` emitting SVG path data, `penTip()` emitting an
animated pen position — validated against real glyph outlines extracted from the
vendored fonts by `truetype.ts` to a sub-2-font-unit join tolerance. It is imported by
nothing but its own test. The machinery for teaching handwriting properly is built,
tested, and dark.

## Three classes of figure

Figures are separated by **what makes them trustworthy**, because that determines how
each is produced, verified, and permitted to be used.

### Class A — script figures (generated, font-derived)

Stroke-order build-ups showing how a letter is formed: the finished glyph outline, the
pen path, the segment labels, the lift points.

- **Source of truth:** `DUCTUS` in `strokes.ts` for the pen path; the vendored font
  outline via `truetype.ts` for the glyph shape.
- **Renderer:** `penPathD()` / `penTip()` composed into SVG through
  `renderToSvgString()` from the existing `@coding-adventures/paint-vm-svg` package.
- **Verification:** the existing `strokes.test.ts` invariants continue to gate the
  data — every pen point on real ink, every intra-stroke join under tolerance, the
  path covering the whole letter — plus provenance (`citation`, `url`) on every entry.
- **Output:** committed SVG, hash-gated exactly like generated `.tex`; CI converts to
  PDF at build time.

> **Hard rule — the glyph monopoly.** Class A is the *only* pipeline permitted to
> depict a letter, glyph, ligature, conjunct, or handwriting stroke, in the book or
> the app. This generalises the argument `truetype.ts` already makes about hand-drawn
> shapes: a subtly wrong Tamil ண looks completely correct to precisely the audience
> that cannot yet read Tamil, so the error would not merely ship — it would ship *as
> the lesson*. No drawn, traced, or model-generated image may render script. Ever.

Authoring cost is real and should not be understated: `DUCTUS` holds one letter today
and needs roughly 190 to cover the nine scripts that already carry cited prose stroke
order. The Dravidian syllabaries (1,378 entries across Kannada, Telugu and Malayalam)
do **not** need per-syllable ductus — they compose, so only base consonants and vowel
signs are authored, and the syllable figure is assembled from those parts.

### Class B — data diagrams (generated)

Etymology and cousin-web trees built from lesson `roots`, sound-articulation diagrams
built from `sounds` ids and the pronunciation reference, gender maps, script-evolution
charts.

- **Source of truth:** the canonical lesson AST. A diagram may assert only what a
  lesson already asserts.
- **Renderer:** deterministic SVG via `paint-vm-svg`, same as Class A.
- **Verification:** hash-gated; every node in a rendered tree must trace to a `roots`
  entry or a declared knowledge atom. A diagram may not introduce a claim.

This matters because the cousin web is the project's signature method, and it is
currently prose-only. An etymology tree is the one diagram this curriculum most
obviously wants.

### Class C — illustrations (authored assets)

Model-generated raster art for scenes, objects, and cultural context — the "colour"
the books currently lack.

- **Storage:** `_assets/illustrations/<track>/`, beside the existing `_fonts/`.
- **Provenance:** each asset carries a sidecar JSON recording generator, model,
  prompt, date, and licence. CI verifies presence, required fields, and a content hash.
- **Determinism:** raster art cannot be regenerated byte-identically, so the *asset*
  is the committed artefact and the hash is the gate — the same posture the repo
  already takes toward vendored fonts.
- **Subject restriction:** non-linguistic subjects only. No script, no glyphs, no
  handwriting, no transliteration, no claim about a language's structure or history.
  Class A holds the glyph monopoly; Class B holds every factual diagram.
- **Budget:** a per-track asset-size cap enforced in CI, so the repository does not
  silently accumulate tens of megabytes of art.

### Licensing

The books are published CC BY-SA 4.0. The licence and attribution stance for
generated illustrations is a **project-owner decision, not a validator default**. The
proposed default is to licence them CC BY-SA 4.0 alongside the book with full
per-asset provenance recorded, following the precedent already set by `_fonts/OFL.txt`.
Until that decision is recorded in `_assets/LICENSE.md`, CI must fail any Class C
asset — an unlicensed image in a freely published book is a real problem, not a
formality.

## Class D — the design system

Figures alone do not make a book colourful. The preambles already load `xcolor` and
`tcolorbox`, so the foundation exists:

- a named palette, defined once and shared across all 20 preambles;
- chapter openers that print the chapter's `canDo` from
  [HL05](./HL05-chapter-capability-and-step-by-step-shape.md);
- restyled `sounds`, `cousinweb`, `culture` and `grammarlens` boxes;
- a figure environment with consistent captioning and placement.

Per-track preambles are currently standalone copies. The palette and figure
environment should land in a shared `_shared/visual.tex` that each preamble inputs,
rather than being copied 20 times — the repo's own lessons warn specifically against
hand-written N-fold families.

## Pipeline changes

1. **Preambles** — add `graphicx` and `\graphicspath`; input the shared visual file.
2. **Renderer** — `book.ts` gains block-level and inline image branches in
   `renderMarkdown` and `renderInlineMarkdown`, plus a filename-safe path escaper
   distinct from `escapeLatexLinkDestination`, with a fail-closed allowlist
   (relative-only, no `..`, extension allowlist) mirroring `safeOutput` in
   `book-cli.ts`.
3. **Figure generation** — a `figure-cli` beside `book-cli`, with the same
   `--write` / `--check` contract and the same manifest-hash discipline.
4. **CI** — add `graphicx.sty` to the `kpsewhich` preflight and `librsvg2-bin` to the
   apt closure for SVG→PDF conversion.

**TikZ is explicitly not adopted.** `texlive-pictures` is deliberately excluded from
the focused CI dependency closure, and pre-rendered vectors are both leaner and more
deterministic than compile-time drawing. Keep the toolchain lean.

## The warning gate

Every track's README and CHANGELOG asserts its book builds with zero missing glyphs,
overfull or underfull boxes, duplicate destinations, hyperref warnings, or font
substitutions. **Nothing enforces this.** CI fails only on hard TeX errors
(`-halt-on-error`) and generated-content drift; no step reads the `.log`.

Floats fight the existing `\raggedbottom` layout and are the classic source of exactly
those warnings, so this spec must not add figures without closing the hole it would
fall through. CI gains a log-scanning step after the `latexmk` loop, checking each
`.log` for `Overfull`, `Underfull`, `Missing character`, hyperref warnings, and
duplicate destinations, compared against a **recorded per-track baseline** so existing
debt is measured rather than newly broken.

## Acceptance criteria

The visual system is complete when every non-Latin track prints a stroke-order figure
for every letter it teaches, generated from font-validated ductus data; the app renders
those same paths from the same source; every Class A and B figure is hash-gated and
regenerable from a clean checkout; every Class C asset carries provenance and a
recorded licence; the shared palette and figure environment live in one file rather
than 20; and the log-scanning gate reports zero new warnings against each track's
baseline.
