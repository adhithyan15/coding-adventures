# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `javascript-tokens` crate: the shared
  JS/TS token vocabulary.
- `EsVersion` (ES1..ES2025) with `es_version_latest` / `default` / `as_str` /
  `all`, `es_version_from_str` (typed status; empty and unknown rejected), and
  `es_version_unknown_message`. Enum values are chronological, so integer
  comparison is chronological comparison.
- `JsSpan` (half-open `[start, end)` u32 range) with `new` / `len` / `is_empty` /
  `eq` / lexicographic `cmp`.
- `JsTokenKind` (20 categories + a borrowed-name `Other`) with `is_trivia`,
  `is_eof`, and `eq` — a trivially-copyable value with no ownership.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC): version strings /
  parsing / ordering, the unknown-version message, Span arithmetic and ordering,
  and TokenKind trivia / eof / equality — mirroring the Rust crate's tests.
