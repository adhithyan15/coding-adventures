# Changelog

## [0.1.0] - 2026-03-25

### Added

- Initial implementation of `commonmark_native` Ruby gem
- `CodingAdventures::CommonmarkNative.markdown_to_html(markdown)` — full
  CommonMark 0.31.2 pipeline with raw HTML passthrough for trusted content
- `CodingAdventures::CommonmarkNative.markdown_to_html_safe(markdown)` — safe
  variant that strips all raw HTML to prevent XSS attacks in web applications
- Zero-dependency implementation via `ruby-bridge` FFI (no Magnus, no rb-sys)
- Cross-platform support: Linux (`.so`), macOS (`.bundle`), Windows (`.dll`)
- `rb_define_module_function` based API for idiomatic Ruby module functions
- Comprehensive test suite with Minitest and SimpleCov coverage tracking,
  covering all CommonMark block elements, inline elements, lists, code blocks,
  raw HTML passthrough, and the safe vs. unsafe contrast
