# Changelog

All notable changes to the display package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- `DisplayConfig` with `DefaultDisplayConfig()`, `VGA80x25`, and `Compact40x10` presets
- `Cell` type representing a character + attribute pair
- `CursorPosition` type for cursor tracking
- `DisplayDriver` with full framebuffer management:
  - `PutChar` with special character handling (newline, carriage return, tab, backspace)
  - `PutCharAt` for direct framebuffer writes with custom attributes
  - `Puts` for string output
  - `Clear` to reset the display
  - `Scroll` to shift rows up
  - `SetCursor` / `GetCursor` for cursor management
  - `GetCell` for reading cell contents
- `DisplaySnapshot` with `Contains()`, `LineAt()`, and `String()` methods
- `MakeAttribute` helper to combine foreground and background colors
- Color constants for all 16 foreground and 8 background VGA colors
- Comprehensive test suite covering all operations, edge cases, and both display configurations
