# FNT00 — Font Parser

## Overview

This spec defines a metrics-only OpenType/TrueType font parser for the
coding-adventures monorepo.

The font parser answers one question:

```text
Given raw font file bytes, what are the numeric metrics I need
to lay out text without touching the OS or browser font stack?
```

It does **not** answer:

- how glyphs look (outline parsing is FNT02)
- how characters map to rendering sequences for complex scripts (shaping is FNT01)
- how pixels are produced from outlines (rasterization is FNT03)
- how a paragraph is broken into lines (Knuth-Plass is FNT04)

### Why parse fonts ourselves?

Every OS ships a font subsystem (Core Text on macOS, DirectWrite on Windows,
FreeType+HarfBuzz on Linux). Those subsystems are excellent for painting — they
apply hinting, subpixel rendering, and platform-specific kerning adjustments.
But they are *terrible* for layout:

1. **DOM reflow cost** — asking the browser DOM for text metrics triggers layout.
   Each call can force a full reflow of the document tree.

2. **Canvas measureText cost** — `ctx.measureText()` avoids DOM reflow but still
   crosses the JS-to-GPU process boundary on many browsers. On low-end devices
   the cold-start cost is measurable even with aggressive caching.

3. **No WASM access** — neither DOM nor Canvas is available inside a WASM module.
   A layout engine written in Rust/WASM cannot call `measureText`.

4. **Platform divergence** — Core Text, DirectWrite, and FreeType produce
   *different* advance widths for the same font on the same codepoint due to
   hinting and rounding. Cross-platform reproducible layout requires our own
   metrics source.

By parsing the font file ourselves we get:

- Exact `unitsPerEm` scaling
- Integer advance widths straight from the `hmtx` table (no hinting applied)
- Kerning pairs from the `kern` table
- The numbers TeX uses — same source of truth as `.tfm` files

The parsing is pure arithmetic on a byte buffer. No OS calls. No heap
allocations beyond the parsed struct. WASM-bake-able: parse at build time,
embed the metrics table as a static constant, ship 6 KB instead of a 200 KB
font file.

---

## OpenType/TrueType Binary Layout

An OpenType font file is a *table database*. Think of it as a flat binary
file where the first few bytes act as a directory telling you where each
named table lives.

```
File layout
───────────────────────────────────────────────
  offset 0    Offset Table         (12 bytes)
  offset 12   Table Record[0]      (16 bytes)
  offset 28   Table Record[1]      (16 bytes)
  ...
  offset 12+16*numTables  (actual table data, any order)
───────────────────────────────────────────────
```

All multi-byte integers in every table are **big-endian** (network byte order).
A compliant reader must byte-swap on little-endian machines (x86, ARM in LE mode).

### Offset Table (bytes 0–11)

| Field          | Type | Offset | Description                                       |
|----------------|------|--------|---------------------------------------------------|
| `sfntVersion`  | u32  | 0      | `0x00010000` = TrueType outlines; `0x4F54544F` ("OTTO") = CFF/PostScript outlines |
| `numTables`    | u16  | 4      | Number of table records that follow               |
| `searchRange`  | u16  | 6      | `(2^floor(log2(numTables))) * 16`                 |
| `entrySelector`| u16  | 8      | `floor(log2(numTables))`                          |
| `rangeShift`   | u16  | 10     | `numTables * 16 - searchRange`                    |

`searchRange`, `entrySelector`, and `rangeShift` are precomputed helpers for
binary search over the table record array. We read but do not rely on them.

### Table Record (16 bytes each, starting at offset 12)

| Field      | Type     | Offset | Description                                |
|------------|----------|--------|--------------------------------------------|
| `tag`      | [u8; 4]  | 0      | ASCII table name, e.g. `b"head"`, `b"cmap"` |
| `checksum` | u32      | 4      | Sum of all u32 words in the table (modulo 2^32) |
| `offset`   | u32      | 8      | Absolute byte offset from start of file    |
| `length`   | u32      | 12     | Byte length of the table                   |

To find a named table:

```
for i in 0..numTables:
    record = bytes[12 + i*16 .. 12 + i*16 + 16]
    if record[0..4] == desired_tag:
        return (offset=u32be(record[8..12]), length=u32be(record[12..16]))
raise FontError::TableNotFound(tag)
```

A production parser would binary-search the records (they are sorted by tag in
spec-compliant fonts). For v0.1.0 a linear scan is acceptable.

---

## Tables (v0.1.0)

### `head` — Font Header

The `head` table holds global font metadata. We need two fields:

| Field              | Type | Offset | Description                                     |
|--------------------|------|--------|-------------------------------------------------|
| `magicNumber`      | u32  | 12     | Must equal `0x5F0F3CF5`. Validates parse.       |
| `unitsPerEm`       | u16  | 18     | Design units per em square. Typically 1000 or 2048. All advance widths and metrics are in these units. |
| `indexToLocFormat` | i16  | 50     | 0 = short offsets in `loca`; 1 = long offsets. Used by `glyf` (FNT02). |

To convert design units to pixels: `pixels = design_units * font_size_px / unitsPerEm`.

For Inter Regular, `unitsPerEm = 2048`. A 16px font at 72 dpi means each
design unit is `16/2048 = 0.0078125` pixels wide.

### `hhea` — Horizontal Header

The `hhea` table describes the horizontal layout parameters of the font.

| Field               | Type | Offset | Description                                               |
|---------------------|------|--------|-----------------------------------------------------------|
| `ascender`          | i16  | 4      | Distance from baseline to top of tallest glyph (positive) |
| `descender`         | i16  | 6      | Distance from baseline to bottom of deepest glyph (negative) |
| `lineGap`           | i16  | 8      | Extra spacing between lines (often 0)                     |
| `numberOfHMetrics`  | u16  | 34     | Number of full (advanceWidth, lsb) records in `hmtx`      |

`ascender - descender + lineGap` gives the natural line height in design units.

### `maxp` — Maximum Profile

The `maxp` table declares the maximum resource requirements for the font.
We only need one field:

| Field       | Type | Offset | Description          |
|-------------|------|--------|----------------------|
| `numGlyphs` | u16  | 4      | Total glyph count    |

The version field at offset 0 is either `0x00005000` (version 0.5, only
`numGlyphs`) or `0x00010000` (version 1.0, full table). Either way
`numGlyphs` is at offset 4.

### `cmap` — Character Map

The `cmap` table maps Unicode codepoints to glyph IDs. It may contain
multiple subtables for different platforms and encodings.

**Preference order for subtable selection:**

1. Platform 3 (Windows), Encoding 1 (Unicode BMP) — Format 4. Use this if present.
2. Platform 0 (Unicode), any encoding — Format 4. Fallback.

#### cmap index structure

At offset 0 of the `cmap` table:

| Field           | Type | Offset | Description              |
|-----------------|------|--------|--------------------------|
| `version`       | u16  | 0      | Always 0                 |
| `numSubtables`  | u16  | 2      | Number of encoding records |

Each encoding record (8 bytes) follows at offset 4:

| Field            | Type | Offset | Description                   |
|------------------|------|--------|-------------------------------|
| `platformID`     | u16  | 0      | 0=Unicode, 1=Macintosh, 3=Windows |
| `encodingID`     | u16  | 2      | Platform-specific encoding    |
| `subtableOffset` | u32  | 4      | Offset *from start of cmap table* |

#### Format 4 subtable

Format 4 encodes the BMP (codepoints 0x0000–0xFFFF) using segments.
Each segment covers a contiguous range of codepoints `[startCode, endCode]`.

At the subtableOffset within the `cmap` table:

| Field              | Type   | Offset           | Description |
|--------------------|--------|------------------|-------------|
| `format`           | u16    | 0                | Must be 4   |
| `length`           | u16    | 2                | Byte length of this subtable |
| `language`         | u16    | 4                | 0 for most fonts |
| `segCountX2`       | u16    | 6                | `segCount * 2` |
| `searchRange`      | u16    | 8                | Binary search helper |
| `entrySelector`    | u16    | 10               | Binary search helper |
| `rangeShift`       | u16    | 12               | Binary search helper |
| `endCode[0..seg]`  | [u16]  | 14               | Inclusive end of each segment |
| `reservedPad`      | u16    | 14 + segCount*2  | Must be 0 |
| `startCode[0..seg]`| [u16]  | 16 + segCount*2  | Inclusive start of each segment |
| `idDelta[0..seg]`  | [i16]  | 16 + segCount*4  | Glyph ID delta per segment |
| `idRangeOffset[0]` | [u16]  | 16 + segCount*6  | Offset into glyphIdArray or 0 |
| `glyphIdArray[]`   | [u16]  | 16 + segCount*8  | Glyph IDs for indirect segments |

#### Format 4 lookup algorithm

```
segCount = segCountX2 / 2

// Step 1: find the segment whose endCode >= codepoint (binary search)
for i in 0..segCount:
    if endCode[i] >= codepoint:
        // Step 2: verify startCode
        if startCode[i] > codepoint:
            return None  // gap between segments — glyph not in font
        // Step 3: resolve glyph ID
        if idRangeOffset[i] == 0:
            glyphId = (codepoint as i32 + idDelta[i] as i32) as u16
        else:
            // Indirect lookup via glyphIdArray
            // idRangeOffset[i] is byte offset *from the address of idRangeOffset[i]*
            // pointing into glyphIdArray
            index = idRangeOffset[i] / 2
                  + (codepoint - startCode[i])
                  - (segCount - i)
            glyphId = glyphIdArray[index]
        if glyphId == 0:
            return None  // explicitly missing
        return glyphId
return None  // codepoint > all endCodes
```

The idRangeOffset trick is one of the trickiest parts of OpenType. The value
at `idRangeOffset[i]` is a byte offset measured *from the memory address of
`idRangeOffset[i]` itself* into the `glyphIdArray`. Since `idRangeOffset` and
`glyphIdArray` are adjacent in memory, the formula `idRangeOffset[i]/2 - (segCount - i)`
converts that self-relative byte offset into an index into `glyphIdArray`.

**Worked example (Inter Regular, letter 'A' = 0x0041):**

1. Scan endCode[] for first value ≥ 0x0041. Suppose segment i=5 has endCode=0x007E.
2. startCode[5] = 0x0020 ≤ 0x0041. ✓
3. idRangeOffset[5] = 0 → glyphId = (0x0041 + idDelta[5]) & 0xFFFF.
4. Suppose idDelta[5] = -29 → glyphId = (65 - 29) = 36. This is the glyph index for 'A'.

### `hmtx` — Horizontal Metrics

The `hmtx` table stores the horizontal advance width and left side bearing
for each glyph.

Structure: two arrays, concatenated:

```
hMetrics[0 .. numberOfHMetrics]    — each entry: advanceWidth (u16), lsb (i16)
leftSideBearings[0 .. numGlyphs - numberOfHMetrics]  — lsb (i16) only
```

For glyph IDs in `[0, numberOfHMetrics)`, both `advanceWidth` and `lsb`
come from `hMetrics[glyphId]`.

For glyph IDs `≥ numberOfHMetrics` (composite glyphs that share the last
advance), `advanceWidth = hMetrics[numberOfHMetrics - 1].advanceWidth` and
`lsb = leftSideBearings[glyphId - numberOfHMetrics]`.

This compression is used for monospaced segments at the end of the glyph set.

### `kern` — Kerning

The `kern` table stores kerning adjustments: horizontal distance corrections
for specific glyph pairs. Kerning makes "AV" look less spaced-out than "AV"
would if both glyphs were just placed advance-width apart.

#### Table structure

At offset 0:

| Field      | Type | Offset | Description          |
|------------|------|--------|----------------------|
| `version`  | u16  | 0      | Must be 0            |
| `nTables`  | u16  | 2      | Number of kern subtables |

Each subtable header (6 bytes):

| Field      | Type | Offset | Description                             |
|------------|------|--------|-----------------------------------------|
| `version`  | u16  | 0      | Subtable version, must be 0             |
| `length`   | u16  | 2      | Byte length of this subtable            |
| `coverage` | u16  | 4      | Bits: format (high 8), flags (low 8)    |

`format = coverage >> 8`. We only handle Format 0.

#### Format 0 subtable

At offset 6 within the subtable:

| Field       | Type | Offset | Description                             |
|-------------|------|--------|-----------------------------------------|
| `nPairs`    | u16  | 0      | Number of kerning pairs                 |
| `searchRange`| u16 | 2      | Binary search helper                    |
| `entrySelector`| u16| 4     | Binary search helper                    |
| `rangeShift`| u16  | 6      | Binary search helper                    |

Each kerning pair record (6 bytes), sorted ascending by composite key:

| Field   | Type | Description                       |
|---------|------|-----------------------------------|
| `left`  | u16  | Left glyph ID                     |
| `right` | u16  | Right glyph ID                    |
| `value` | i16  | Kerning adjustment in design units (negative = tighter) |

#### Format 0 lookup algorithm

```
composite_key = ((left_glyph_id as u32) << 16) | (right_glyph_id as u32)

// Binary search on pairs sorted by composite_key
lo = 0
hi = nPairs
while lo < hi:
    mid = (lo + hi) / 2
    pair_key = (pairs[mid].left as u32) << 16 | (pairs[mid].right as u32)
    if pair_key == composite_key:
        return pairs[mid].value
    elif pair_key < composite_key:
        lo = mid + 1
    else:
        hi = mid
return 0  // pair not found → no kerning adjustment
```

**Worked example (Inter Regular, 'A'+'V'):**

'A' is glyph 36, 'V' is glyph 57 (example values; actual IDs vary).
`composite_key = (36 << 16) | 57 = 0x00240039`.
Binary search finds the pair record with value = -140 design units.
At 16px with unitsPerEm=2048: `-140 * 16 / 2048 ≈ -1.1 px`.

### `name` — Naming Table

The `name` table stores human-readable strings: family name, style, copyright,
version, etc. Strings are referenced by platform, encoding, language, and
nameID.

We need:
- nameID 1 — Font Family Name
- nameID 2 — Font Subfamily Name (e.g. "Regular", "Bold")

**Preferred lookup**: Platform 3 (Windows), Encoding 1 (Unicode BMP),
Language 0x0409 (English US). Strings are encoded as **UTF-16 Big-Endian**.

#### name table structure

| Field       | Type | Offset | Description              |
|-------------|------|--------|--------------------------|
| `format`    | u16  | 0      | 0 or 1                   |
| `count`     | u16  | 2      | Number of name records   |
| `stringOffset`| u16| 4      | Offset to string storage (from start of name table) |

Each name record (12 bytes) at offset 6:

| Field          | Type | Offset | Description             |
|----------------|------|--------|-------------------------|
| `platformID`   | u16  | 0      |                         |
| `encodingID`   | u16  | 2      |                         |
| `languageID`   | u16  | 4      |                         |
| `nameID`       | u16  | 6      |                         |
| `length`       | u16  | 8      | Byte length of string   |
| `stringOffset` | u16  | 10     | Offset from stringOffset base |

To decode a string:

```
absolute_offset = name_table_offset + stringOffset + record.stringOffset
raw_bytes = file[absolute_offset .. absolute_offset + record.length]
// For platform 3 encoding 1: decode as UTF-16 BE
string = utf16be_decode(raw_bytes)
```

### `OS/2` — OS/2 and Windows Metrics

The `OS/2` table provides typographic metrics preferred by modern renderers over
the older `hhea` values. Available from OS/2 version 0; `xHeight` and
`capHeight` added in version 2.

| Field              | Type | Offset | Description                                       |
|--------------------|------|--------|---------------------------------------------------|
| `version`          | u16  | 0      | Table version (0–5)                               |
| `typoAscender`     | i16  | 68     | Preferred ascender (positive)                     |
| `typoDescender`    | i16  | 70     | Preferred descender (negative)                    |
| `typoLineGap`      | i16  | 72     | Preferred line gap                                |
| `xHeight`          | i16  | 86     | Height of lowercase 'x' (version ≥ 2 only)       |
| `capHeight`        | i16  | 88     | Height of uppercase 'H' (version ≥ 2 only)       |

If `version < 2`, `xHeight` and `capHeight` are absent. Return `null`/`None`
for these fields in that case.

---

## Interface Contract

This section is language-agnostic. Each implementation maps these types and
functions to the idiomatic forms of its language.

### Types

```
FontMetrics:
  units_per_em:    u16     — design units per em
  ascender:        i16     — from OS/2.typoAscender (or hhea.ascender if OS/2 absent)
  descender:       i16     — from OS/2.typoDescender (or hhea.descender)
  line_gap:        i16     — from OS/2.typoLineGap (or hhea.lineGap)
  x_height:        i16?    — null/None if OS/2 version < 2
  cap_height:      i16?    — null/None if OS/2 version < 2
  num_glyphs:      u16     — from maxp
  family_name:     string  — nameID 1, platform 3 encoding 1
  subfamily_name:  string  — nameID 2, platform 3 encoding 1

GlyphMetrics:
  advance_width:       u16  — horizontal advance in design units
  left_side_bearing:   i16  — space before the glyph in design units
```

### Error type

```
FontError:
  InvalidMagic          — sfntVersion is not a known value
  TableNotFound(tag)    — required table missing from directory
  BufferTooShort        — byte slice ended before expected field
  UnsupportedCmapFormat — no Format 4 subtable for platform 3 encoding 1
```

### Functions

```
load(bytes: &[u8]) → Result<FontFile, FontError>
```
Parse the font bytes and return an opaque handle. Must validate:
- `sfntVersion` is `0x00010000` or `0x4F54544F`
- `head.magicNumber` == `0x5F0F3CF5`
- Required tables present: head, hhea, maxp, cmap, hmtx

---

```
font_metrics(font: &FontFile) → FontMetrics
```
Return global font metrics. Prefers OS/2 typographic values; falls back to
hhea values if OS/2 is absent.

---

```
glyph_id(font: &FontFile, codepoint: u32) → Option<u16>
```
Map a Unicode codepoint to a glyph ID via the Format 4 cmap subtable.
Returns `None`/`null` if the codepoint is not in the font.

Codepoints above 0xFFFF are outside Format 4 range — return `None`.

---

```
glyph_metrics(font: &FontFile, glyph_id: u16) → Option<GlyphMetrics>
```
Return advance width and left side bearing for a glyph ID.
Returns `None` if `glyph_id >= num_glyphs`.

---

```
kerning(font: &FontFile, left: u16, right: u16) → i16
```
Return the kerning adjustment for the given pair in design units.
Returns 0 if no kern table exists or the pair is not found.
Negative values mean tighter spacing (most common case for kerned pairs).

---

## Test Font

All implementations use **Inter Regular** from the Inter font family by
Rasmus Andersson (SIL Open Font License 1.1).

Source: https://github.com/rsms/inter/releases

Known values for Inter Regular (used as test assertions):

| Property | Expected value |
|---|---|
| `units_per_em` | 2048 |
| `family_name` | `"Inter"` |
| `subfamily_name` | `"Regular"` |
| `num_glyphs` | ≥ 100 (typically 2500+) |
| `glyph_id('A')` (U+0041) | non-null |
| `glyph_id(0x0000)` (NUL) | null/None — not in BMP coverage |
| `kerning(glyph_id('A'), glyph_id('V'))` | negative integer |

The exact `advance_width` for 'A' should match the `ttx -t hmtx Inter-Regular.ttf`
output. Verify with:

```sh
ttx -t hmtx Inter-Regular.ttf
# Look for GID matching 'A' glyph → advanceWidth="NNN"
```

---

## Package Matrix

| Language   | Directory                                    | Module/Namespace                        |
|------------|----------------------------------------------|-----------------------------------------|
| Rust       | `code/packages/rust/font-parser/`            | `font_parser`                           |
| TypeScript | `code/packages/typescript/font-parser/`      | `@coding-adventures/font-parser`        |
| Python     | `code/packages/python/font-parser/`          | `font_parser`                           |
| Ruby       | `code/packages/ruby/font_parser/`            | `CodingAdventures::FontParser`          |
| Go         | `code/packages/go/font-parser/`              | `fontparser`                            |
| Elixir     | `code/packages/elixir/font_parser/`          | `CodingAdventures.FontParser`           |
| Lua        | `code/packages/lua/font-parser/`             | `coding_adventures.font_parser`         |
| Perl       | `code/packages/perl/font-parser/`            | `CodingAdventures::FontParser`          |
| Swift      | `code/packages/swift/font-parser/`           | `FontParser`                            |

### Native Extension Wrappers

These packages wrap the Rust `font-parser` crate using hand-rolled
zero-dependency bridges. No PyO3, magnus, or napi-rs.

| Package               | Bridge         | Directory                              |
|-----------------------|----------------|----------------------------------------|
| `font-parser-python`  | `python-bridge` | `code/packages/rust/font-parser-python/` |
| `font-parser-ruby`    | `ruby-bridge`   | `code/packages/rust/font-parser-ruby/`  |
| `font-parser-node`    | `node-bridge`   | `code/packages/rust/font-parser-node/`  |

---

## FNT Series Roadmap

| Spec  | Package             | Description                                              |
|-------|---------------------|----------------------------------------------------------|
| FNT00 | `font-parser`       | Metrics tables: head/hhea/maxp/cmap/hmtx/kern/name/OS2   |
| FNT01 | `font-shaper`       | GSUB substitutions, ligatures, context rules             |
| FNT02 | `glyph-parser`      | glyf/CFF outline parsing, Bézier contours                |
| FNT03 | `glyph-rasterizer`  | Scanline fill, anti-aliasing                             |
| FNT04 | `font-layout`       | Knuth-Plass paragraph line breaking using FNT00 metrics  |

FNT00 is the foundation: every package in this series depends on the ability
to load a font file and read its metrics tables.

---

## Implementation Notes

### Big-endian reading helpers

Every implementation needs these primitives:

```
u8(buf, offset)  = buf[offset]
u16be(buf, offset) = (buf[offset] as u16) << 8 | buf[offset+1] as u16
i16be(buf, offset) = u16be(buf, offset) as i16   // reinterpret bits
u32be(buf, offset) = (buf[offset] as u32) << 24
                   | (buf[offset+1] as u32) << 16
                   | (buf[offset+2] as u32) << 8
                   |  buf[offset+3] as u32
i32be(buf, offset) = u32be(buf, offset) as i32
```

Bounds check before every read. Return `FontError::BufferTooShort` if
`offset + width > buf.len()`.

### Tag comparison

Tags are 4 ASCII bytes. Compare as byte arrays, not as integers (avoids
endian confusion):

```
tag_matches(buf, offset, tag: [u8; 4]) = buf[offset..offset+4] == tag
```

### UTF-16 BE decoding

For the `name` table, strings with platform 3 encoding 1 are UTF-16 BE.
Decode by reading pairs of bytes as big-endian u16 code units, then
converting to the language's native string type.

Surrogates (code units 0xD800–0xDFFF) form pairs encoding codepoints above
U+FFFF (Supplementary Multilingual Plane). For v0.1.0, font family names
fit entirely in BMP — treat unpaired surrogates as replacement characters
(U+FFFD) and continue.

### Required test coverage

Every language implementation must include tests that:

1. Load `Inter-Regular.ttf` bytes from the test fixtures directory
2. Assert `font_metrics().units_per_em == 2048`
3. Assert `font_metrics().family_name == "Inter"`
4. Assert `glyph_id(0x0041)` ('A') is non-null
5. Assert `glyph_metrics(glyph_id('A')).advance_width > 0`
6. Assert `kerning(glyph_id('A'), glyph_id('V')) < 0`
7. Assert `glyph_id(0xFFFF)` returns null (not in font)
8. Assert `load` returns an error for a zero-byte buffer
9. Assert `glyph_metrics` returns null/None for glyph_id >= num_glyphs

Coverage threshold: ≥ 80% lines (target 95%+).
