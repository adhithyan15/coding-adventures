# font-parser (C++)

A **metrics-only OpenType/TrueType font parser** — header-only, ISO C++17. A
faithful port of the Rust [`font-parser`](../../rust/font-parser) crate, in
namespace `ca::font_parser`. It reads the subset of font tables needed to
*measure text* — no OS font stack, no outlines, no shaping, no rasterization.

## What it reads

`head` (unitsPerEm), `hhea` (ascender/descender/lineGap/numberOfHMetrics),
`maxp` (numGlyphs), `cmap` (Format 4 Unicode → glyph id), `hmtx` (advance +
LSB per glyph), `kern` (Format 0 pairs), `name` (family/subfamily, UTF-16 BE →
UTF-8), and `OS/2` (typo metrics, x/cap height for version ≥ 2).

## API

```cpp
#include "font_parser.hpp"
namespace fp = ca::font_parser;

auto font = fp::FontFile::load(bytes);   // throws fp::FontError on failure
auto m = font.metrics();                 // m.units_per_em, m.family_name, ...
if (auto gid = font.glyph_id('A')) {
    auto gm = font.glyph_metrics(*gid);  // std::optional<GlyphMetrics>
}
std::int16_t k = font.kerning(a, v);     // design units, 0 if none
```

- `FontFile::load` throws `fp::FontError` where the Rust `load` returns
  `Result`.
- `metrics()` returns `FontMetrics` (with `std::optional<int16_t>` x/cap height
  and `std::string` names); `glyph_id` / `glyph_metrics` return `std::optional`;
  `kerning` returns `int16_t`. Every read is bounds-checked.

## Building

Builds through the shared [`iso-harness`](../../c/iso-harness) engine under every
ISO C++ compiler on `PATH` with `-std=c++17 -pedantic-errors -Wall -Wextra
-Werror`:

```sh
sh BUILD          # POSIX: g++ and/or clang++
```

Each compiler prints `N checks, 0 failed`. Tests build a synthetic in-memory
font (no external `.ttf` fixture). Verified clean under ASan + UBSan, including
a truncation fuzz over every prefix of a valid font.
