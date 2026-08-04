# Changelog

All notable changes to `trie` (C) are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the Rust `trie` crate: `trie_init`, `insert`,
  `search`, `contains_key`, `delete` (with node pruning), `starts_with`,
  `foreach`/`foreach_prefix` (sorted visitor enumeration), `longest_prefix_match`,
  `len`, `is_empty`. Byte-keyed (256-way) `int`-valued nodes.
- Tests (via the shared `iso-harness`) covering insert/search/contains,
  overwrite, delete with pruning, prefix queries, sorted enumeration, and
  longest-prefix match — compiled and run under GCC, Clang, and MSVC with strict
  ISO-conformance flags.
