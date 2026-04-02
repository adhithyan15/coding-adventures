# font-parser-ruby (Rust)

Ruby C extension wrapping the Rust `font-parser` core. Part of the FNT series.

Exposes `load`, `font_metrics`, `glyph_id`, `glyph_metrics`, and `kerning`
as module functions on `CodingAdventures::FontParserNative`.

## Quick start

```ruby
require "font_parser_native"

data = File.binread("Inter-Regular.ttf")
font = CodingAdventures::FontParserNative.load(data)

m = CodingAdventures::FontParserNative.font_metrics(font)
puts m[:units_per_em]   # 2048
puts m[:family_name]    # "Inter"

gid_a = CodingAdventures::FontParserNative.glyph_id(font, 0x0041)
gm    = CodingAdventures::FontParserNative.glyph_metrics(font, gid_a)
puts gm[:advance_width]        # e.g. 1401
puts gm[:left_side_bearing]    # e.g. 7

kern = CodingAdventures::FontParserNative.kerning(
  font, gid_a, CodingAdventures::FontParserNative.glyph_id(font, 0x0056)
)
# 0 — Inter v4.0 uses GPOS, not the legacy kern table
```

## How it works

`load()` uses `rb_data_object_wrap` to store a `Box<FontFile>` inside a
Ruby `Data` object (`CodingAdventures::FontParserNative::FontFile`).
Ruby's GC calls the registered `dfree` function when the object is
garbage-collected, which runs `Box::from_raw` and drops the Rust memory.

## Building

```bash
cargo build -p font-parser-ruby --release
```

## License

MIT
