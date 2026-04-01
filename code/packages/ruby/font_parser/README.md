# coding_adventures_font_parser

A metrics-only OpenType/TrueType font parser written in pure Ruby with **zero
runtime dependencies**. Part of the FNT series (see
[`code/specs/FNT00-font-parser.md`](../../../../specs/FNT00-font-parser.md)).

## Why?

Text layout engines — line-breakers, typographers, renderers — need font metrics
(advance widths, kerning pairs, vertical extents) to position glyphs without
relying on the OS or browser font stack. This library exposes exactly those
metrics in a form that is:

- **Portable** — pure Ruby, no C extensions
- **Predictable** — same bytes in → same numbers out on every platform
- **Foundation-ready** — designed as the metrics layer for a future Knuth-Plass
  paragraph layout engine (`FNT04`)

## Installation

```ruby
# Gemfile
gem "coding_adventures_font_parser"
```

Or install directly:

```
gem install coding_adventures_font_parser
```

## Quick start

```ruby
require "coding_adventures/font_parser"

FP = CodingAdventures::FontParser

# 1. Load font bytes from disk (binary mode is required)
bytes = File.binread("/path/to/MyFont.ttf")
font  = FP.load(bytes)

# 2. Global metrics
m = FP.font_metrics(font)
puts m.family_name      # => "Inter"
puts m.units_per_em     # => 2048
puts m.ascender         # => 1984
puts m.descender        # => -494
puts m.x_height         # => 1082  (nil if OS/2 version < 2)
puts m.cap_height       # => 1456  (nil if OS/2 version < 2)
puts m.num_glyphs       # => 2548

# 3. Glyph lookup (Unicode codepoint → glyph index)
gid_a = FP.glyph_id(font, 0x0041)  # 'A'
gid_v = FP.glyph_id(font, 0x0056)  # 'V'

# 4. Per-glyph metrics
gm = FP.glyph_metrics(font, gid_a)
puts gm.advance_width      # e.g. 1401
puts gm.left_side_bearing  # e.g. 7

# 5. Kerning (returns 0 when no kern table or pair absent)
k = FP.kerning(font, gid_a, gid_v)
puts k  # negative for A+V in fonts with a kern table
```

## API reference

### `CodingAdventures::FontParser.load(bytes) → FontFile`

Parses raw font bytes (a binary-encoded `String` or any object that responds to
`bytesize` / `getbyte`). Returns an opaque `FontFile` on success.

**Raises** `FontError` on failure. Check `err.kind` for the error category:

| `kind` | Meaning |
|---|---|
| `"BufferTooShort"` | Input is too short to be a valid font |
| `"InvalidMagic"` | sfntVersion magic not recognised |
| `"TableNotFound"` | Required table (`head`, `hhea`, `maxp`, `cmap`, `hmtx`) missing |
| `"ParseError"` | Table data is structurally invalid |

---

### `font_metrics(font) → FontMetrics`

Returns a frozen value object with fields:

| Field | Type | Description |
|---|---|---|
| `units_per_em` | Integer | Design units per em |
| `ascender` | Integer | Typographic ascender (signed font units) |
| `descender` | Integer | Typographic descender (signed, usually negative) |
| `line_gap` | Integer | Extra inter-line gap |
| `x_height` | Integer\|nil | Height of lowercase 'x' (OS/2 v2+) |
| `cap_height` | Integer\|nil | Height of capital letters (OS/2 v2+) |
| `num_glyphs` | Integer | Total glyph count |
| `family_name` | String | e.g. `"Inter"` (UTF-8, `"(unknown)"` if absent) |
| `subfamily_name` | String | e.g. `"Regular"` (UTF-8, `"(unknown)"` if absent) |

---

### `glyph_id(font, codepoint) → Integer | nil`

Maps a Unicode codepoint (Integer) to a glyph index via the `cmap` Format 4
BMP subtable. Returns `nil` for codepoints outside the BMP (`> 0xFFFF`),
negative values, or codepoints not covered by the font.

---

### `glyph_metrics(font, glyph_id) → GlyphMetrics | nil`

Returns a frozen value object with:

| Field | Type | Description |
|---|---|---|
| `advance_width` | Integer | Horizontal advance (font units) |
| `left_side_bearing` | Integer | Left-side bearing (signed font units) |

Returns `nil` for out-of-range or negative glyph IDs.

---

### `kerning(font, left_glyph_id, right_glyph_id) → Integer`

Returns the kern value (signed font units) for the ordered glyph pair from the
`kern` Format 0 subtable, or `0` if the table is absent or the pair is not listed.

> **Note:** Many modern fonts (including Inter v4.0) use GPOS for kerning instead
> of the legacy `kern` table. `kerning()` only reads `kern` Format 0 and will
> return `0` for GPOS-only fonts.

## Architecture

The parser is a single-module pure Ruby file
(`lib/coding_adventures/font_parser.rb`) — no external dependencies, no C
extensions. Every parsing decision is explained inline in the source code in
Knuth-style literate programming style.

**Parsing pipeline:**

```
bytes
  └─► offset table   sfntVersion (magic check), numTables
        └─► table records  tag → (offset, length) for each named table
              └─► head    unitsPerEm, indexToLocFormat
              └─► hhea    ascender, descender, lineGap, numberOfHMetrics
              └─► maxp    numGlyphs
              └─► cmap    Format 4 BMP subtable (platform 3, enc 1)
              └─► hmtx    advanceWidth + lsb per glyph
              └─► kern    Format 0 sorted pairs (optional)
              └─► name    family/subfamily UTF-16 BE strings (optional)
              └─► OS/2    xHeight, capHeight (optional, version ≥ 2)
```

## Development

```
bundle install
bundle exec rake test        # run 30 tests
bundle exec standardrb       # lint (must be clean)
```

## In the FNT series

| Spec | Package | Adds |
|---|---|---|
| **FNT00** | **font-parser** | **Metrics tables (this package)** |
| FNT01 | font-shaper | GSUB substitutions, ligatures |
| FNT02 | glyph-parser | glyf/CFF outline parsing |
| FNT03 | glyph-rasterizer | Scanline fill, anti-aliasing |
| FNT04 | font-layout | Knuth-Plass line breaking |

## License

MIT
