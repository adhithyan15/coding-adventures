# Changelog

All notable changes to the C `display` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `display` crate — an 80x25 VGA
  text-mode framebuffer simulation over caller-owned memory.
- `display_make_attribute` + the `DisplayColor` palette; `display_init` /
  `display_wrap`; writing (`display_put_char` with `\n`/`\r`/`\t`/backspace
  handling, line wrap and scroll; `display_put_char_at`; `display_puts`);
  screen/cursor management (`display_clear`, `display_scroll`,
  `display_set_cursor`, `display_get_cursor`, `display_get_cell`); and a
  `DisplaySnapshot` text view (`display_snapshot` + `_free` / `_contains` /
  `_line_at` / `_to_padded`).
- The framebuffer is borrowed, not owned (mirroring the Rust `&mut [u8]`); every
  access is bounds-checked against the borrowed length so an undersized buffer
  degrades to a no-op instead of overflowing. Snapshot allocations and the
  `to_padded` size are guarded against `size_t` overflow.
- 1171 checks mirroring the Rust crate's own unit tests (attributes, control
  characters, wrapping, scrolling with attribute preservation, clear, snapshot
  trimming/contains/padding, cursor clamping, and the full 256-byte sweep), run
  under every available C compiler via the shared `iso-harness`; the suite also
  passes clean under AddressSanitizer + UndefinedBehaviorSanitizer.
