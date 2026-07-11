# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `suffix-tree` crate (DT15), in
  namespace `ca`.
- `ca::suffix_tree` with `build` / `build_ukkonen`, `search`,
  `count_occurrences`, `longest_repeated_substring`, `all_suffixes`,
  `node_count`, `text_len`, `text`.
- `ca::longest_common_substring` free function using rolling dynamic programming.
- `search` / `longest_common_substring` accept `std::string_view`; byte-oriented
  like `std::string`.
- Tests pinned to the crate's assertions plus edge cases, run under GCC and Clang
  via `iso-harness`.
