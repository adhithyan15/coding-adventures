# font-parser (C)

A **metrics-only OpenType/TrueType font parser** in pure ISO C17. A faithful
port of the Rust [`font-parser`](../../rust/font-parser) crate. It reads the
subset of font tables needed to *measure text* — without touching the OS font
stack, parsing glyph outlines, shaping, or rasterizing.

## What it reads

| Table  | What we read                                      |
|--------|---------------------------------------------------|
| `head` | `unitsPerEm`                                       |
| `hhea` | ascender / descender / lineGap / numberOfHMetrics |
| `maxp` | `numGlyphs`                                        |
| `cmap` | Format 4 Unicode → glyph id mapping                |
| `hmtx` | advance width + left side bearing per glyph        |
| `kern` | Format 0 kerning pairs (binary-searched)           |
| `name` | family / subfamily name (UTF-16 BE → UTF-8)        |
| `OS/2` | typo ascender/descender/lineGap, x/cap height (v≥2)|

## API

```c
#include "font_parser.h"

FontFile *f = NULL;
if (font_load(bytes, len, &f) == FONT_OK) {
    FontMetrics m;
    font_metrics(f, &m);          /* m.units_per_em, m.family_name, ... */

    uint16_t gid;
    if (font_glyph_id(f, 'A', &gid)) {
        GlyphMetrics gm;
        font_glyph_metrics(f, gid, &gm);       /* gm.advance_width */
    }
    int16_t k = font_kerning(f, a, v);         /* design units, 0 if none */
    font_free(f);
}
```

- `font_load` returns a `FontError` (0 = `FONT_OK`) and, on success, an owned
  `FontFile` (free with `font_free`).
- `font_metrics`, `font_glyph_id`, `font_glyph_metrics`, `font_kerning`, and the
  bounds-checked big-endian helpers `font_read_u16` / `_i16` / `_u32`.

Every multi-byte read is bounds-checked with overflow-safe arithmetic, so a
truncated or corrupt file yields an error or a `0`/false return, never an
out-of-bounds access. Verified clean under ASan + UBSan, the macOS `leaks` tool
(0 leaks), and a truncation fuzz (every prefix of a valid font parses safely).

### Divergences from the Rust

- `FontError::TableNotFound` carries the missing table's name in Rust; this port
  returns the plain `FONT_ERR_TABLE_NOT_FOUND` code.
- Name strings are decoded UTF-16 BE → UTF-8 into fixed 128-byte buffers
  (truncated if longer).

## Building

Builds through the shared [`iso-harness`](../iso-harness) engine under every ISO
C compiler on `PATH` with `-std=c17 -pedantic-errors -Wall -Wextra -Werror`:

```sh
sh BUILD          # POSIX: gcc and/or clang
```

Each compiler prints `N checks, 0 failed`. Tests build a synthetic in-memory
font (no external `.ttf` fixture needed).
