# Changelog — font-parser-ruby (Rust)

## [Unreleased]

### Fixed — the extension could not be loaded, and every call misread its arguments

The crate had only ever been compiled, never `require`d.  Loading it into a real
Ruby (3.3.6) exposed three defects, all fixed:

- **`undefined symbol: rb_data_object_get`.**  It is a `static inline` function
  in Ruby's headers, so no libruby exports it.  `unwrap_font` now reads the
  pointer with `ruby_bridge::unwrap_data` after the `rb_obj_is_kind_of` check.
- **Receiver and arguments were swapped.**  `rb_define_module_function` calls the
  C function as `(self, arg1, ..., argN)`; the functions were written
  `(arg1, ..., argN, self)`, so every call raised
  `TypeError: no implicit conversion of Module into String`.
- **`nil` was a hard-coded constant.**  `QNIL = 0x08` is wrong on Ruby 3.2+
  (`Qnil` is `0x04`), so a missing glyph came back as a bogus object.  Returns
  now use `ruby_bridge::nil_value()`.

`FontFile` also calls `rb_undef_alloc_func`, silencing Ruby's "undefining the
allocator of T_DATA class" warning and making `FontFile.new` raise.

### Added — real Windows link and a smoke test on every OS

- `build.rs` links Ruby's DLL on Windows: it writes a `.def` file from the
  `rb_*` declarations in this crate and ruby-bridge (scanned, not hand-listed),
  and has MSVC `lib.exe` (from PATH, else via vswhere) turn it into an import
  library; the GNU target links RubyInstaller's `.dll.a` directly.  macOS gets
  `-undefined dynamic_lookup`.
- `tests/smoke_test.rb` requires the built extension into Ruby and checks Inter's
  metrics, glyph lookup, `nil` for a missing glyph, kerning, GC survival and the
  error paths.  `BUILD` and `BUILD_windows` were `echo` ("compile-only crate");
  they now build into the package's own `target/`, run `cargo clippy -D
  warnings`, and run the smoke test.

## [0.1.0] — 2026-04-01

### Added

- Initial release — Ruby C extension wrapping the Rust `font-parser` core.

- **`CodingAdventures::FontParserNative.load(data) → FontFile`** — Parses a
  font from a binary String. Returns an opaque Data object. Raises
  `RuntimeError` on parse failure.

- **`CodingAdventures::FontParserNative.font_metrics(font) → Hash`** — Returns
  a Hash with Symbol keys: `:units_per_em`, `:ascender`, `:descender`,
  `:line_gap`, `:x_height` (Integer | nil), `:cap_height` (Integer | nil),
  `:num_glyphs`, `:family_name`, `:subfamily_name`.

- **`CodingAdventures::FontParserNative.glyph_id(font, cp) → Integer | nil`**
  — Maps a Unicode codepoint to a glyph ID. Returns `nil` if unmapped.

- **`CodingAdventures::FontParserNative.glyph_metrics(font, gid) → Hash | nil`**
  — Returns a Hash with `:advance_width` and `:left_side_bearing`. Returns
  `nil` for out-of-range glyph IDs.

- **`CodingAdventures::FontParserNative.kerning(font, left, right) → Integer`**
  — Returns the kern value for a pair of glyph IDs; 0 when not found.

### Implementation notes

- Uses `rb_data_object_wrap` (via ruby-bridge) to store `Box<FontFile>` in a
  Ruby Data object. GC-safe: destructor calls `Box::from_raw` to drop Rust mem.
- Data pointer stored/read at RData offset 4 (word 4 = `data` field in RData).
- Hash keys are Ruby Symbols via `rb_intern` + `rb_id2sym`.
- `crate-type = ["cdylib"]`, lib name `font_parser_native` — matches the
  `Init_font_parser_native` entry point and the `.so`/`.bundle` file name.
