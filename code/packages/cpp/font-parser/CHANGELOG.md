# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C++17, header-only port of the Rust `font-parser` crate in namespace
  `ca::font_parser`: a metrics-only OpenType/TrueType font parser
  (head/hhea/maxp/cmap/hmtx/kern/name/OS-2) for measuring text.
- `FontFile::load` (throws `FontError` where the Rust returns `Result`),
  `metrics()` (with `std::optional<int16_t>` x/cap height and `std::string`
  names decoded UTF-16 BE → UTF-8), `glyph_id` / `glyph_metrics` returning
  `std::optional`, `kerning` returning `int16_t`, and bounds-checked big-endian
  read helpers in `detail`. RAII throughout (`std::vector` buffer). Every read
  is bounds-checked; verified clean under ASan + UBSan and a truncation fuzz
  over every prefix of a valid font.
- 36 checks run under every ISO C++ compiler via the shared `iso-harness`,
  exercising the full parser against a self-contained synthetic in-memory font
  (no external `.ttf` fixture).
