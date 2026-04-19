# FNT05 — Coding Adventures Test Font

## Overview

FNT05 defines a **homegrown test font** for the coding-adventures
stack: a small, license-clean, code-generated TrueType font with
deliberately round numeric metrics, committed as a fixture and used
by every downstream consumer that needs deterministic text output.

The font exists for three reasons:

1. **Testable metrics.** Round `unitsPerEm`, round ascent/descent,
   round advance widths. Assertions become exact: "the word 'Hello'
   at 16px is exactly 80 pixels wide." No approximate comparisons,
   no "within ε" tolerances for rendering tests.

2. **License cleanliness.** No SIL OFL vendoring. No dependency on
   a system font we can't guarantee is present on CI. The font is
   a first-party asset of the repo, under the same license as
   everything else.

3. **Pedagogy.** The repo's standing practice is to build
   everything from first principles. Shipping our own font — even
   a simple one — aligns with that, and exercises `font-parser`
   (FNT00), `glyph-parser` (FNT02), and `glyph-rasterizer` (FNT03)
   against a font whose every byte we understand.

FNT05 is **not a display typeface**. It will not win awards. It is
a pixel-geometric sans-serif designed to be legible at small sizes,
unambiguous under cell-based rasterization, and trivial to extend.
Think "Silkscreen meets Pico-8 meets the Knuth literate style" —
charmingly technical, not artful.

---

## Scope

FNT05 specifies:

- The font's **design grid and metrics** (concrete numeric values).
- The font's **glyph coverage** in tiers (v0.1 → v0.3).
- The **source file format** that encodes each glyph's pixel
  geometry.
- The **generator tool** that compiles the source file into a
  valid TrueType binary.
- The **fixture protocol** — how downstream packages consume the
  font in their tests.
- **Licensing** (SIL OFL 1.1, to match the open font norm without
  being a vendor relationship).

FNT05 does NOT specify:

- How the font is rendered. That is the paint backend's job
  (FNT03 + P2D05, etc.).
- How the font is loaded at runtime. That is the test harness's
  job; fonts are read from the committed TTF bytes directly.
- The font's use in production. FNT05 is a **test fixture**. It
  is not intended as a fallback display face.

---

## Why not vendor an existing open font?

Vendoring (say) Inter or Noto Sans into the repo would solve the
"CI has a font" problem and would look prettier. It was considered
and rejected for these reasons:

- **Size.** Modern variable fonts are 200–500 KB. FNT05 at the
  Tier 1 coverage is expected to be under 8 KB.
- **License propagation.** Every vendored font's license must be
  preserved and reproduced in certain downstream contexts. Our
  own license is already threaded through every package.
- **Metrics rounding.** Real display fonts use
  `unitsPerEm = 2048` with metrics like `ascender = 2728`. Round
  numbers are more pleasant to reason about in tests.
- **Feature surface.** Real fonts have thousands of glyphs, GSUB
  tables, GPOS tables, variable axes, color layers. We get to
  choose what tables exist; missing tables force test coverage of
  "font lacks X" code paths.
- **Substitutability.** A vendored font is immutable. A homegrown
  font can grow: "add one codepoint for the new test" is
  straightforward.

A vendored font may still land in the repo later as a **separate**
fixture for testing real-world font behavior (big cmap, variable
axes, OpenType features). FNT05 is the **minimal** fixture.

---

## Design grid and metrics

All values in TrueType design units. The fundamental grid is
**16 × 16 design-pixel squares** inside an em of **1024 design
units**, so each design-pixel is **64 design units**.

```
                   1024 design units  (1 em)
         ┌──────────────────────────────────────────┐
         │                                          │
         │  Ascender top  y = +768 (= +12 px)       │
         │─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─  │
         │  Cap height    y = +576 (= +9 px)        │
         │─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─  │
         │  x-height      y = +384 (= +6 px)        │
         │─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─  │
         │  Baseline      y =   0                   │
         │═══════════════════════════════════════  │
         │  Descender bot y = -256 (= -4 px)        │
         │                                          │
         └──────────────────────────────────────────┘

Advance width:  default 512 design units (= 8 px, half an em)
                (monospace; every glyph except whitespace variants
                and explicitly-narrow punctuation shares this value)
```

### The numbers

| Metric          | Design units | Design pixels | Notes                                 |
|-----------------|--------------|---------------|---------------------------------------|
| `unitsPerEm`    | 1024         | 16            | Power of 2; nice for shifts and tests |
| `ascender`      |  768         | 12            | OS/2.typoAscender + hhea.ascender     |
| `descender`     | -256         | -4            | Signed per font convention; TXT01 returns `abs = 256` |
| `lineGap`       |    0         |  0            | No extra gap; consumers add CSS line-height if needed |
| `xHeight`       |  384         |  6            | OS/2.sxHeight                         |
| `capHeight`     |  576         |  9            | OS/2.sCapHeight                       |
| Default advance |  512         |  8            | Monospace default                     |
| Narrow advance  |  256         |  4            | For quote, period, comma — half the default |

Natural line height = ascender − descender + lineGap = 768 − (−256) + 0 = 1024 = 1 em.

At a 16-pixel font size:
- 1 em = 16 pixels
- 1 design pixel = 1 rendered pixel
- Glyph advance = 8 pixels for normal glyphs, 4 pixels for narrow
- Line height = 16 pixels exactly

This is the **canonical 16-pixel case**. Other sizes scale
linearly: at 32px, glyph advance = 16px, line height = 32px.
Deterministic by construction.

---

## Glyph coverage — tiered

### Tier A (v0.1.0) — Basic Latin

**Range:** U+0020 (space) through U+007E (tilde). 95 glyphs total.

Includes:
- Whitespace: space (U+0020).
- Digits: `0 1 2 3 4 5 6 7 8 9` (U+0030–U+0039).
- Uppercase letters: `A`–`Z` (U+0041–U+005A).
- Lowercase letters: `a`–`z` (U+0061–U+007A).
- Punctuation: `! " # $ % & ' ( ) * + , - . / : ; < = > ? @ [ \ ] ^ _ \` { | } ~`.

Plus the mandatory `.notdef` glyph at glyph_id 0 (a simple
outline-rectangle placeholder, 8 px wide, ascender-tall).

Coverage: sufficient for rendering plain English Markdown. Enough
to exercise every shaping path in TXT02 and every rendering path
in a paint backend.

### Tier B (v0.2.0) — Latin-1 Supplement

Adds U+00A0–U+00FF: accented Latin characters (`à`, `é`, `ñ`, `ü`,
etc.), inverted punctuation (`¿`, `¡`), and common symbols (`©`,
`®`, `°`, `±`, `§`, `¶`).

Coverage: sufficient for rendering European-language Markdown.
Tests the TXT02 cluster-offset handling on multi-byte UTF-8
characters.

### Tier C (v0.3.0) — Typographic extras

Adds select characters from other Unicode blocks:

| Codepoint | Glyph              | Purpose                     |
|-----------|--------------------|-----------------------------|
| U+2013    | en-dash            | CommonMark emits these      |
| U+2014    | em-dash            | CommonMark emits these      |
| U+2018    | left single quote  | Smart quotes                |
| U+2019    | right single quote | Smart quotes / apostrophe   |
| U+201C    | left double quote  | Smart quotes                |
| U+201D    | right double quote | Smart quotes                |
| U+2026    | horizontal ellipsis| `...` → `…`                 |
| U+00B7    | middle dot         | List bullets (alt)          |
| U+2022    | bullet             | List bullets                |

(Some of these are in Latin-1 Supplement; listed here for
completeness.)

Coverage: sufficient for rendering "typographically correct"
English Markdown — curly quotes, real dashes, ellipsis character.

### Beyond Tier C

Later additions (not specified in FNT05 v1) could include:
- Box-drawing characters (U+2500–U+257F) for terminal-style
  rendering tests.
- A handful of mathematical symbols (U+00D7, U+00F7, U+2212) for
  LaTeX rendering proofs of concept.
- A single emoji-range character (with a simple black outline)
  to test COLR/sbix fallback paths.

These are out of scope for FNT05 v1. File follow-up specs when
a concrete consumer needs them.

---

## Source file format

Each glyph is specified in a declarative source file. The format
is **TOML**, chosen because every supported language in the repo
has a TOML parser available.

### File layout

```
fonts/test-font/
├── metadata.toml        # global font metadata (name, metrics)
├── glyphs/
│   ├── 0020.toml        # one file per codepoint (hex, zero-padded)
│   ├── 0021.toml
│   ├── ...
│   └── 007E.toml
└── build.log            # generator's deterministic build record
```

One file per glyph keeps diffs reviewable: a change to the "A"
glyph touches exactly `glyphs/0041.toml` and nothing else.

### `metadata.toml`

```toml
name            = "CodingAdventuresFixture"
family          = "Coding Adventures Fixture"
subfamily       = "Regular"
version         = "0.1.0"
copyright       = "(c) 2026 coding-adventures contributors, SIL OFL 1.1"
license         = "OFL-1.1"

[metrics]
units_per_em    = 1024
ascender        = 768
descender       = -256
line_gap        = 0
x_height        = 384
cap_height      = 576
default_advance = 512
narrow_advance  = 256
```

These numeric values are canonical — changing them is a
breaking change to every test that asserts on them.

### Per-glyph file (e.g. `glyphs/0041.toml` for 'A')

```toml
codepoint = 0x0041        # U+0041 LATIN CAPITAL LETTER A
advance   = 512           # design units (use "narrow_advance"
                          # for narrow glyphs, or specify explicitly)

# Contour format: one or more closed polygons.
# Coordinates are in design-pixel grid units (0..16 horizontally,
# -4..12 vertically where 0 is baseline). The generator scales by
# 64 to produce design-unit coordinates in the TTF.

[[contour]]
points = [
  [1, 0], [3, 0], [3, 2], [6, 2], [6, 0], [8, 0],
  [8, 9], [6, 11], [3, 11], [1, 9],
]
# Each point is [x, y]. Coordinates are connected by straight
# segments in order. The contour closes automatically from the
# last point back to the first.

# A glyph may have multiple contours (holes, like in 'O' or '8').
# Inner contours must wind opposite the outer contour per
# TrueType fill-rule conventions.
```

Curved shapes (where desired for visual appeal) are encoded as
polygons with dense enough vertices that they read as curves at
rendering size. Cubic or quadratic Bezier segments are
**explicitly not supported** in FNT05 v1 — polygons are easier
to author, diff-review, and compile correctly. Later versions
may add Bezier support.

### Kerning

FNT05 v1 is **monospace**. No kerning pairs are specified and
the `kern` table is omitted. This simplifies the TTF emitter and
removes a variable from early tests. A future "Proportional"
subfamily can introduce kerning without invalidating FNT05 v1's
fixture guarantees.

---

## The generator tool

A companion **package** (not defined in this spec in detail)
compiles the source files into a valid TrueType binary. Proposed
naming: `test-font-compiler`. Proposed language: **Rust**
primary, with language-specific wrappers as needed.

### Inputs

- The `metadata.toml` file.
- All files under `glyphs/`.

### Output

- A single `CodingAdventuresFixture-Regular.ttf` binary, written
  to a version-stamped output directory.
- A `build.log` recording the generator version, input file
  hashes, and output byte count — for deterministic-build
  auditing.

### Required tables in the output TTF

| Table | Purpose                                                                 |
|-------|-------------------------------------------------------------------------|
| head  | Global font header: magic, unitsPerEm, bbox                             |
| hhea  | Horizontal header: ascender, descender, lineGap, numberOfHMetrics       |
| maxp  | Maximum profile: numGlyphs                                              |
| hmtx  | Horizontal metrics: per-glyph advance + lsb                             |
| cmap  | Character map: Format 4 subtable covering U+0020 to the tier's max     |
| glyf  | Glyph outlines (simple glyphs only — no composites in v1)              |
| loca  | Glyph offsets (short format: 16-bit offsets × 2)                        |
| name  | Font name: family, subfamily, unique id, full name, version             |
| OS/2  | OS/2 metrics: typoAscender, typoDescender, sxHeight, sCapHeight, etc.  |
| post  | PostScript info: glyph names (version 2.0)                              |

No `kern`, no `GSUB`, no `GPOS`, no `GDEF`, no `cvt`/`fpgm`/`prep`
(no hinting), no `name` platform/encoding records beyond the
minimum (Windows 3/1/0x409 Unicode BMP English-US).

Total table count: **10**. A well-formed modern font commonly has
20+ tables. FNT05 is deliberately smaller.

### Determinism requirement

The generator MUST produce byte-identical output for a given
source tree. This is enforced by:

- Sorting all glyph records by codepoint before emitting.
- Never embedding timestamps (`head.created` and `head.modified`
  are set to a fixed epoch, `0x00000000 0x00000000`, in Jan 1904
  of the TrueType epoch).
- Using a deterministic pattern for optional fields (e.g., empty
  glyphs get `numberOfContours = 0` with zero-length instruction
  data, not a missing entry in `glyf`).
- Computing checksums in a documented order.

The generated TTF is committed to the repo at
`code/fonts/test-font/CodingAdventuresFixture-Regular.ttf`. CI
verifies that re-running the generator produces the same bytes.

---

## Design principles — what makes it "beautiful"

Beauty in a pixel-geometric font is achieved through **consistency
and clarity**, not through expressive typography. The FNT05 design
guidelines:

1. **One stroke thickness.** Every stroke is exactly 2 design
   pixels (128 design units) wide. No optical adjustments, no
   tapering.

2. **Rectilinear where possible, 45° where required, curves
   where essential.** `A V W M N` are built from straight
   segments with clean diagonals. `O C G S` use faceted octagons
   (8-point polygons) to suggest curvature. No partial pixels.

3. **Consistent vertical alignment.** Every uppercase letter
   reaches `cap_height`. Every lowercase letter without an
   ascender reaches `x_height`. Ascenders (`b d f h k l`) reach
   `ascender`. Descenders (`g j p q y`) extend exactly to
   `descender`.

4. **Consistent horizontal centering.** Monospace means every
   glyph is centered inside its 8-pixel advance box with
   consistent left and right side-bearings (conventionally 1
   pixel each, leaving a 6-pixel ink area).

5. **Unambiguous under rendering.** Glyphs that commonly confuse
   at small sizes (`0` vs `O`, `1` vs `l` vs `I`) are drawn
   deliberately distinct. `0` has a diagonal slash. `l` has a
   slight foot. `I` has serifs.

6. **Joy in the technical constraint.** The aesthetic goal is
   "this looks like it was designed to be read by machines, and
   happens to be charming" — similar to early VGA fonts or the
   Knuth Computer Modern pixel ancestors.

A specimen of the `A`, `a`, `O`, and `g` glyphs is provided in the
FNT05 package README for reference — these four establish the
design language.

---

## Fixture protocol

FNT05 provides the following **fixture API** to downstream packages.

### File access

The TTF bytes are committed at:
```
code/fonts/test-font/CodingAdventuresFixture-Regular.ttf
```

Packages that need the font read this file at test time. Each
language's test harness provides a helper:

```
# Python
from coding_adventures_test_fixtures import load_test_font
font_bytes = load_test_font()

# Rust
use coding_adventures_test_fixtures::load_test_font;
let font_bytes = load_test_font();

# TypeScript
import { loadTestFont } from "@coding-adventures/test-fixtures";
const fontBytes = loadTestFont();
```

These helpers resolve the committed TTF path relative to the
workspace root and return its bytes. They are trivial — a few
lines of per-language code — and live in a shared
`test-fixtures` package.

### Canonical numeric expectations

A companion JSON file at:
```
code/fonts/test-font/expected-metrics.json
```

encodes the exact metric values every downstream package should
assert on:

```json
{
  "units_per_em": 1024,
  "ascender": 768,
  "descender_abs": 256,
  "line_gap": 0,
  "x_height": 384,
  "cap_height": 576,
  "family_name": "Coding Adventures Fixture",
  "glyph_advances": {
    "A": 512,
    "a": 512,
    ".": 256,
    ",": 256
  }
}
```

A TXT01 conformance test loads this JSON and calls each
`FontMetrics` method, asserting equality. Any divergence indicates
either a bug in that language's `font-parser` port or a drift in
FNT05 itself (caught at CI).

### Glyph sample strings

FNT05 additionally publishes **canonical test strings** with
pre-computed shaped outputs for TXT02 conformance:

| String    | Expected length (px at 16px) | Notes                   |
|-----------|------------------------------|-------------------------|
| `"A"`     | 8                            | One monospace glyph     |
| `"Hello"` | 40                           | 5 glyphs × 8px          |
| `".,"`    | 8                            | 2 narrow glyphs × 4px   |
| `""`      | 0                            | Empty string edge case  |

Implementations assert these lengths exactly. Failure indicates
either TXT02 or FNT05 is broken.

---

## Licensing

FNT05 is licensed under **SIL OFL 1.1** (SIL Open Font License),
the de facto license for open fonts. This is a deliberate choice:

- OFL is explicitly drafted for fonts. It's the license Google
  Fonts, Fontsource, and most open foundries use. Tooling knows
  how to handle it.
- OFL permits use in any document (rendering Markdown with FNT05
  does not make the output licensed under OFL).
- OFL requires preservation of the reserved font name. FNT05's
  reserved name is `"Coding Adventures Fixture"` — derivatives
  must not claim that name.

The OFL text lives at `code/fonts/test-font/LICENSE`. The
generator embeds a compact copyright notice in the TTF's `name`
table (nameID 0 and nameID 13).

---

## Testing the font itself

Before FNT05 is declared stable, the following tests must pass:

1. **Round-trip through `font-parser`.** The generated TTF parses
   cleanly, `font_parser::font_metrics()` returns the canonical
   values from `expected-metrics.json`, and
   `font_parser::glyph_id(codepoint)` returns a nonzero ID for
   every codepoint in the current tier.

2. **Cross-language parity.** Every language's `font-parser` port
   parses the TTF identically. Tested by the shared fixture
   suite.

3. **Deterministic build.** Running the generator twice produces
   byte-identical TTFs.

4. **Reference rasterization.** A known pixel grid rendered from
   FNT05 via FNT03 (glyph-rasterizer) matches a committed
   pixel-array reference for the 16px baseline case. This
   detects accidental geometry changes.

5. **TTF validity.** The TTF passes `fontTools`' validator (run
   from a CI job if `fontTools` is available; non-blocking if
   not installed).

Coverage target: **100%** of the glyph set for the current tier,
and 100% of the metric fields in `expected-metrics.json`.

---

## Non-goals

**Bezier curves.** FNT05 v1 uses straight-line polygons only.
Bezier support can be added later without breaking the fixture
contract (fixture bytes change, but the metric assertions remain).

**Variable axes.** No design-axis variation. The font has one
weight, one width, one optical size.

**OpenType features.** No GSUB, no GPOS. The `kern` table is
omitted. Nothing for TXT02 to silently ignore and nothing for
TXT04 to show off.

**Hinting.** No `cvt`, `fpgm`, or `prep` tables. Low-resolution
rendering quality is acceptable for a test fixture; hinting would
add orders of magnitude of complexity to the generator.

**Subsetting.** The TTF ships as one file with everything. No
on-demand subsetting. A 4 KB file does not need subsetting.

**Production use.** FNT05 is NOT intended to be the default font
for anything shipped to end users. It is a test asset.

---

## Relationship to sibling specs

| Spec  | Relationship                                                                   |
|-------|--------------------------------------------------------------------------------|
| FNT00 | Upstream consumer — `font-parser` must parse FNT05's output without errors     |
| FNT02 | Upstream consumer — `glyph-parser` must read FNT05's `glyf` table correctly    |
| FNT03 | Upstream consumer — `glyph-rasterizer` rasterizes FNT05 glyphs for snapshot tests |
| TXT01 | Primary test bed — FNT05's canonical metrics JSON is the cross-language conformance fixture |
| TXT02 | Primary test bed — FNT05's canonical test strings give exact-length shaped-run assertions |
| P2D05 | Downstream — paint backends render FNT05 in integration tests                  |
| HF06  | Content hashing of the generated TTF bytes for `font-parser:` font_ref stability checks |

FNT05 is the **keystone fixture**. Almost every text-related test
in the repo ends up touching it.

---

## Roadmap

| Version | Milestone                                                                      |
|---------|--------------------------------------------------------------------------------|
| v0.1.0  | Tier A (Basic Latin, 95 glyphs) + generator + fixture helpers                  |
| v0.1.1  | Polish: per-glyph unit tests, README with glyph specimen, LICENSE              |
| v0.2.0  | Tier B (Latin-1 Supplement, +96 glyphs)                                        |
| v0.3.0  | Tier C (typographic extras; smart quotes, dashes, ellipsis)                    |
| v0.4.0  | Optional: add kern table (~20 hand-picked pairs) for TXT02 kerning tests       |
| v1.0.0  | Declared stable; byte-identical generator output frozen as the fixture baseline |

Each version is a separate implementation PR that extends the
source tree and regenerates the TTF. Spec revisions (to FNT05
itself) happen when design decisions change, not when glyphs are
added within an already-specified tier.

---

## Open questions

- **Font name.** "Coding Adventures Fixture" is the working name.
  Alternatives considered: "Grid Sans", "CA Pixel", "Knuth Fixture".
  Decide when the generator implementation PR lands.

- **Reserved font name in OFL.** Do we reserve the full family
  name or just a distinctive prefix? OFL mechanics: reserving a
  longer string is more protective but restricts forks.
  Recommendation: reserve `"Coding Adventures Fixture"` exactly.

- **Committing the TTF vs. generating at test time.** The spec
  commits the TTF. An alternative is to regenerate at test time,
  keeping only the source TOML in the repo. Trade-off: committed
  TTF is simpler for CI but requires regeneration discipline on
  every source change. Recommendation: commit the TTF;
  determinism check catches drift.

- **Glyph specimen images in README.** Generating preview PNGs of
  each glyph at 1×, 2×, 4× zoom would be helpful for design
  review but adds binary assets. Consider for v0.2.
