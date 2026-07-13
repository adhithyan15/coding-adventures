# Changelog

All notable changes to the C++ `display` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial header-only pure-ISO C++17 port of the Rust `display` crate
  (namespace `ca::display`) — an 80x25 VGA text-mode framebuffer simulation over
  a caller-owned `std::vector<std::uint8_t>&`.
- `make_attribute` + `COLOR_*` palette; `DisplayConfig::default_config()` /
  `::compact()`; a `DisplayDriver` (clearing constructor + non-clearing
  `wrap`) with `put_char` (control-character handling, wrap, scroll),
  `put_char_at`, `puts`, `clear`, `scroll`, `set_cursor`, `get_cursor`,
  `get_cell`; and a value-semantic `DisplaySnapshot`
  (`to_string_padded`/`contains`/`line_at`).
- Every framebuffer access is bounds-checked against the viewed length, so an
  undersized buffer degrades to a no-op instead of overflowing.
- 1159 checks mirroring the Rust crate's own unit tests, run under every
  available C++ compiler via the shared `iso-harness`; the suite also passes
  clean under AddressSanitizer + UndefinedBehaviorSanitizer.
