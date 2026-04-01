# coding-adventures-font-parser

Metrics-only OpenType/TrueType font parser. Zero dependencies, pure Python.

## What it does

Reads raw font bytes and returns numeric metrics from 8 OpenType/TrueType
tables — enough to measure text without the OS font stack.

| Table  | Fields read                                              |
|--------|----------------------------------------------------------|
| `head` | `unitsPerEm`, `magicNumber` validation                   |
| `hhea` | ascender, descender, lineGap, numberOfHMetrics           |
| `maxp` | numGlyphs                                                |
| `cmap` | Format 4 (Unicode BMP → glyph ID), platform 3 / enc 1   |
| `hmtx` | advanceWidth + leftSideBearing per glyph                 |
| `kern` | Format 0 sorted pair table                               |
| `name` | family / subfamily names (UTF-16 BE)                     |
| `OS/2` | typoAscender/Descender/LineGap, xHeight, capHeight       |

## Usage

```python
from font_parser import load, font_metrics, glyph_id, glyph_metrics, kerning

data = open("Inter-Regular.ttf", "rb").read()
font = load(data)

m = font_metrics(font)
print(m.units_per_em)    # 2048
print(m.family_name)     # "Inter"
print(m.x_height)        # positive int

gid_a = glyph_id(font, ord("A"))  # 'A' = U+0041
gid_v = glyph_id(font, ord("V"))

gm = glyph_metrics(font, gid_a)
print(gm.advance_width)  # design units

kern = kerning(font, gid_a, gid_v)

# Convert to pixels at 16px:
kern_px = kern * 16 / m.units_per_em
```

## Design

`load()` copies the font bytes into a `FontFile` and pre-parses the table
directory. All metric queries are O(1) byte reads (`struct.unpack_from`) or
O(log N) binary searches. No external packages needed.

Uses Python's `struct` module with `">H"` / `">h"` / `">I"` format codes
for big-endian decoding.

## Running tests

```bash
uv venv && uv pip install -e ".[dev]"
pytest tests/ -v
```
