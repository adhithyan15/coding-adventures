# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C++17 header-only port of the Rust `javascript-tokens` crate,
  in namespace `ca::jstokens`.
- `EsVersion` (enum class, ES1..ES2025) with `as_str`, `es_version_all`,
  `es_version_latest` / `default`, `es_version_try_parse` (`std::optional`) and
  `es_version_parse` (throws `UnknownEsVersion`); chronological ordering via the
  enum's built-in relational operators.
- `Span` (half-open `[start, end)` u32 range) with `constexpr` `make` / `len` /
  `is_empty` and the full comparison operators (lexicographic).
- `TokenKind` (20 categories + an `Other` name) with `is_trivia`, `is_eof`,
  `operator==`, and `operator<` (usable as a `std::map` key).
- Tests via the shared `iso-harness` (GCC, Clang, MSVC): version strings /
  parsing / ordering, the unknown-version message, Span arithmetic / ordering /
  const construction, and TokenKind trivia / eof / equality / map-key usage.
