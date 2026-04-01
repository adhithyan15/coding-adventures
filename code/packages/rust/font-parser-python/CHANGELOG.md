# Changelog — font-parser-python (Rust)

## [0.1.0] — 2026-04-01

### Added

- Initial release — Python C extension wrapping the Rust `font-parser` core.

- **`load(data: bytes) -> capsule`** — Parses a font from a `bytes` object.
  Returns a `PyCapsule` holding a heap-allocated `Box<FontFile>`. Raises
  `ValueError` on parse failure, `TypeError` if `data` is not bytes.

- **`font_metrics(font) -> dict`** — Returns a `dict` with keys:
  `units_per_em`, `ascender`, `descender`, `line_gap`, `x_height` (int | None),
  `cap_height` (int | None), `num_glyphs`, `family_name`, `subfamily_name`.

- **`glyph_id(font, codepoint: int) -> int | None`** — Maps a Unicode
  codepoint (BMP only) to a glyph ID. Returns `None` if unmapped.

- **`glyph_metrics(font, glyph_id: int) -> dict | None`** — Returns a
  `dict` with `advance_width` and `left_side_bearing`. Returns `None`
  for out-of-range glyph IDs.

- **`kerning(font, left: int, right: int) -> int`** — Returns the kern
  value for a pair of glyph IDs; 0 when no pair is found.

### Implementation notes

- Uses `PyCapsule` (PEP 384 stable API) to wrap the `Box<FontFile>` pointer.
  The capsule destructor calls `Box::from_raw` to drop the `FontFile` when
  Python GC collects the handle.
- `PyBytes_AsStringAndSize` borrows directly from the Python bytes buffer —
  no copy; `fp::load` synchronously parses and copies what it needs.
- Inline `extern "C"` declarations for `PyCapsule_*`, `PyBytes_*`, `PyDict_*`,
  `PyLong_AsLong` — not modifying python-bridge for these niche additions.
- `crate-type = ["cdylib"]`, lib name `font_parser_native` — matches the
  `PyInit_font_parser_native` entry point and the `.so`/`.pyd` file name.
