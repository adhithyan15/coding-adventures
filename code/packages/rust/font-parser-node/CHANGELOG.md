# Changelog — font-parser-node (Rust)

## [0.1.0] — 2026-04-01

### Added

- Initial release — Node.js N-API addon wrapping the Rust `font-parser` core.

- **`load(buffer: Buffer) → FontFile`** — Parses a font from a Node.js
  Buffer. Returns an opaque JS object wrapping `Box<FontFile>`. Throws an
  Error on parse failure or wrong argument type.

- **`fontMetrics(font) → object`** — Returns an object with camelCase keys:
  `unitsPerEm`, `ascender`, `descender`, `lineGap`, `xHeight` (number | null),
  `capHeight` (number | null), `numGlyphs`, `familyName`, `subfamilyName`.

- **`glyphId(font, codepoint: number) → number | null`** — Maps a Unicode
  codepoint to a glyph ID. Returns `null` if unmapped.

- **`glyphMetrics(font, glyphId: number) → object | null`** — Returns an
  object with `advanceWidth` and `leftSideBearing`. Returns `null` for
  out-of-range glyph IDs.

- **`kerning(font, left: number, right: number) → number`** — Returns the
  kern value for a pair of glyph IDs; 0 when not found.

### Implementation notes

- Uses `napi_wrap` with a GC finalizer to store `Box<FontFile>` in a JS object.
  The finalizer calls `Box::from_raw` to drop Rust memory when GC collects.
- `napi_get_buffer_info` extracts raw bytes from a Node.js Buffer without copy.
- `napi_new_instance` + stored constructor VALUE (`FONT_FILE_CTOR`) creates
  new FontFile JS instances.
- `napi_register_module_v1` entry point — the ABI-stable N-API module
  registration function (replaces the old `NODE_MODULE_INIT` / `napi_module`).
- Targets N-API v4 (Node.js 10.16+) for maximum compatibility.
- `crate-type = ["cdylib"]`, lib name `font_parser_native` — rename to
  `font_parser_native.node` after `cargo build --release`.
