# font-parser

Metrics-only OpenType/TrueType font parser. Zero dependencies, no `unsafe`,
WASM-bake-able.

This is the Rust reference implementation for the FNT00 series.

## What it does

An OpenType font file is a binary *table database*. The first few bytes list
all the named tables and where they are. This crate finds the tables you need
to measure text — advance widths, kerning, ascenders, descenders — and
returns plain Rust structs. No rendering, no shaping, no OS calls.

It also reads the **OpenType `MATH` table** — the constants a maths typesetter
needs (axis height, fraction rule thickness, numerator/denominator shifts,
radical gaps, script shifts) and per-glyph italic corrections. Most fonts have
no `MATH` table; `math_constants` returns `Ok(None)` for those, distinguishing
*absent* from *corrupt*.

## What it does not do

- Glyph outline parsing (FNT02 — `glyph-parser`)
- Text shaping / GSUB ligatures (FNT01 — `font-shaper`)
- Pixel rasterization (FNT03 — `glyph-rasterizer`)
- Knuth-Plass line breaking (FNT04 — `font-layout`)

## Usage

```rust
use font_parser::{load, font_metrics, glyph_id, glyph_metrics, kerning};

let bytes = std::fs::read("Inter-Regular.ttf").unwrap();
let font = load(&bytes).unwrap();

let m = font_metrics(&font);
println!("unitsPerEm = {}", m.units_per_em); // 2048 for Inter
println!("family     = {}", m.family_name);   // "Inter"

// Unicode codepoint → glyph ID
let gid_a = glyph_id(&font, 'A' as u32).unwrap();
let gid_v = glyph_id(&font, 'V' as u32).unwrap();

// Per-glyph horizontal metrics
let gm = glyph_metrics(&font, gid_a).unwrap();
println!("A advance_width = {}", gm.advance_width);

// Kerning pair adjustment (negative = tighter)
let kern = kerning(&font, gid_a, gid_v);
println!("kern(A, V) = {} design units", kern); // negative for Inter

// Convert to pixels at 16px:
// pixels = design_units * font_size_px / units_per_em
let kern_px = kern as f32 * 16.0 / m.units_per_em as f32;
println!("kern(A, V) at 16px = {:.2} px", kern_px); // ≈ -1.1 px
```

## Tables parsed

| Table  | Fields read                                              |
|--------|----------------------------------------------------------|
| `head` | `unitsPerEm`, `magicNumber` (validation)                 |
| `hhea` | `ascender`, `descender`, `lineGap`, `numberOfHMetrics`   |
| `maxp` | `numGlyphs`                                              |
| `cmap` | Format 4 subtable (platform 3 / encoding 1)              |
| `hmtx` | `advanceWidth`, `leftSideBearing` per glyph              |
| `kern` | Format 0 sorted pair table                               |
| `name` | nameID 1 (family), nameID 2 (subfamily), UTF-16 BE       |
| `OS/2` | `typoAscender/Descender/LineGap`, `xHeight`, `capHeight` |

## Design

`FontFile` stores a copy of the font bytes and a pre-parsed table directory.
All metric queries are O(1) byte reads or O(log N) binary searches. No heap
allocation occurs during queries — just integer arithmetic over a `&[u8]`.

The crate has zero runtime dependencies. It compiles to WASM without any
JavaScript glue code. The design is inspired by TeX's `.tfm` files: the metrics
are parsed once, stored as integers, and used purely arithmetically during layout.

## Verifying the MATH table

The `MATH` reader is checked against **fontTools**, an independent
implementation, reading the same font. `tests/fixtures/stix-two-math.MATH` is
the real table extracted from STIX Two Math (OFL 1.1 — see the fixture's
`PROVENANCE.md`), not bytes we wrote.

That distinction is the point: a hand-built fixture and a hand-written reader
share their author's understanding of the specification, so a wrong stride or a
transposed field is invisible — both halves are wrong the same way and agree
perfectly. The expected values come from someone else's implementation.

The specific hazard, pinned by its own test: almost every `MathConstants` entry
is a `MathValue` — an `int16` followed by an `Offset16` device table. Reading
them as bare `int16` *appears* to work, because the first field really is the
value, while halving the stride so every constant after the first is wrong.
