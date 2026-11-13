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

## Testing

There are no Rust unit tests: the shim is only meaningful inside Ruby, so the
test requires the built library into a real interpreter and calls it.

```bash
cargo build --release -p font-parser-ruby --target-dir target
ruby tests/smoke_test.rb
```

Each crate builds into its own `target/` (`--target-dir target`) because font-parser-node,
font-parser-python and font-parser-ruby all emit a cdylib named `font_parser_native`.
On Windows, `build.rs` writes a `.def` file from the `rb_*` declarations and has
MSVC `lib.exe` build an import library for Ruby's DLL (set `RUBY` to pick a
specific interpreter).

## License

MIT
