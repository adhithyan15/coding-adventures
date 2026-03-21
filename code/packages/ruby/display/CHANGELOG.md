# Changelog

All notable changes to the display package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- `DisplayConfig` with `VGA_80X25` and `COMPACT_40X10` presets
- `Cell` value object representing a character + attribute pair
- `CursorPosition` value object for cursor tracking
- `DisplayDriver` with full framebuffer management:
  - `put_char` with special character handling (newline, carriage return, tab, backspace)
  - `put_char_at` for direct framebuffer writes with custom attributes
  - `puts_str` for string output
  - `clear` to reset the display
  - `scroll` to shift rows up
  - `set_cursor` / `get_cursor` for cursor management
  - `get_cell` for reading cell contents
- `DisplaySnapshot` with `contains?`, `line_at`, and `to_s` methods
- `Display.make_attribute` helper to combine foreground and background colors
- Color constants for all 16 foreground and 8 background VGA colors
- Comprehensive minitest test suite with SimpleCov coverage
