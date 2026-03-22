# Changelog

All notable changes to the display crate will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- `DisplayConfig` with `Default` trait and `compact()` constructor
- `Cell` struct representing a character + attribute pair
- `CursorPosition` struct for cursor tracking
- `DisplayDriver` with full framebuffer management:
  - `put_char` with special character handling (newline, carriage return, tab, backspace)
  - `put_char_at` for direct framebuffer writes with custom attributes
  - `puts` for string output
  - `clear` to reset the display
  - `scroll` to shift rows up
  - `set_cursor` / `get_cursor` for cursor management
  - `get_cell` for reading cell contents
- `DisplaySnapshot` with `contains()`, `line_at()`, and `to_string_padded()` methods
- `make_attribute` helper to combine foreground and background colors
- Color constants for all 16 foreground and 8 background VGA colors
- 55 unit tests + 2 doc-tests covering all operations and edge cases
