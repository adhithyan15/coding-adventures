# Changelog

All notable changes to `trie` (C++) are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the Rust `trie` crate:
  `ca::trie<V>` with `insert`, `search` (`std::optional`), `contains_key`,
  `erase` (with pruning), `starts_with`, `words_with_prefix`, `all_words`,
  `keys`, `longest_prefix_match`, `size`, `empty`. `std::map`-backed nodes give
  sorted enumeration.
- Tests (via the shared `iso-harness`) covering insert/search/contains,
  overwrite, erase with pruning, prefix queries, sorted enumeration, and
  longest-prefix match — compiled and run under GCC, Clang, and MSVC with strict
  ISO-conformance flags.
