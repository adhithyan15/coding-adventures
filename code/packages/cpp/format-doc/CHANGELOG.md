# Changelog

All notable changes to the C++ `format-doc` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-13

### Added

- Initial header-only pure-ISO C++17 port of the Rust `format-doc` crate
  (namespace `ca::format_doc`) — a Wadler-style document algebra for
  pretty-printers.
- Document builders: `nil`, `text` (CRLF/CR-normalising, splits embedded
  newlines into hardlines), `concat` (flattens nested concats, drops nils),
  `join`, `group`, `indent`, `line`, `softline`, `hardline`, `if_break`,
  `annotate`. `Doc` is immutable with structural sharing via `std::shared_ptr`
  (mirroring the Rust `Arc`).
- `DocAnnotation` is a `std::variant<std::string, std::int64_t, bool, Null>`;
  the `Doc` node type is a `std::variant`; `layout_doc` throws
  `std::invalid_argument` on `print_width == 0` (mirroring the Rust panic).
- `layout_doc` (a stack-machine interpreter whose `group` flat/broken decision
  uses an O(work) `fits` look-ahead that borrows the parent stack) and
  `render_text`. Adjacent spans coalesce only when their annotations match;
  `visible_width` counts UTF-8 code points.
- 59 checks mirroring the Rust crate's own unit tests, run under every available
  C++ compiler via the shared `iso-harness`; the suite also passes clean under
  AddressSanitizer + UndefinedBehaviorSanitizer.
