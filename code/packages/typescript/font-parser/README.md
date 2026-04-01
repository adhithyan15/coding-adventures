# @coding-adventures/font-parser

Metrics-only OpenType/TrueType font parser. Zero dependencies, pure TypeScript,
WASM-bake-able.

This is the TypeScript implementation of the FNT00 series, mirroring the Rust
reference crate `font-parser`.

## What it does

Reads a `Uint8Array` of raw font bytes and returns numeric metrics from 8
OpenType/TrueType tables:

| Table  | Fields read                                              |
|--------|----------------------------------------------------------|
| `head` | `unitsPerEm`, `magicNumber` validation                   |
| `hhea` | ascender, descender, lineGap, numberOfHMetrics           |
| `maxp` | numGlyphs                                                |
| `cmap` | Format 4 (Unicode BMP → glyph ID), platform 3 / enc 1   |
| `hmtx` | advanceWidth + leftSideBearing per glyph                 |
| `kern` | Format 0 sorted pair table                               |
| `name` | family / subfamily, UTF-16 BE                            |
| `OS/2` | typoAscender/Descender/LineGap, xHeight, capHeight       |

## What it does not do

- Glyph outline parsing (FNT02)
- Text shaping / GSUB ligatures (FNT01)
- Pixel rasterization (FNT03)
- Knuth-Plass line breaking (FNT04)

## Usage

```typescript
import { load, fontMetrics, glyphId, glyphMetrics, kerning } from "@coding-adventures/font-parser";
import { readFileSync } from "node:fs";

const bytes = new Uint8Array(readFileSync("Inter-Regular.ttf"));
const font = load(bytes);

const m = fontMetrics(font);
console.log(m.unitsPerEm);    // 2048 for Inter
console.log(m.familyName);    // "Inter"
console.log(m.xHeight);       // positive integer (from OS/2 table)

// Codepoint → glyph ID
const gidA = glyphId(font, 0x0041)!; // 'A'
const gidV = glyphId(font, 0x0056)!; // 'V'

// Per-glyph metrics
const gm = glyphMetrics(font, gidA)!;
console.log(gm.advanceWidth); // design units

// Kerning (requires a kern table; Inter uses GPOS)
const kern = kerning(font, gidA, gidV);

// Convert to pixels at 16px:
const kernPx = kern * 16 / m.unitsPerEm;
```

## Design

`FontFile` stores a `DataView` over a copy of the font bytes plus a pre-parsed
table directory. All metric queries are O(1) reads or O(log N) binary searches.
No DOM access, no `canvas.measureText`, no OS calls.

Uses `DataView` with `false` (big-endian) for all reads — correct on all
platforms without manual byte swapping.
