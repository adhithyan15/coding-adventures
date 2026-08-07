# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C17 port of the Rust `suffix-tree` crate (DT15): a suffix index over
  a stored copy of the text (byte-oriented).
- `suffix_tree_build` / `suffix_tree_free`, `suffix_tree_search` /
  `suffix_tree_count_occurrences`, `suffix_tree_longest_repeated_substring`,
  `suffix_tree_suffix`, `suffix_tree_node_count`, `suffix_tree_text_len`.
- `suffix_longest_common_substring` free function using rolling dynamic
  programming (two rows).
- Overflow-safe search bound (`start <= len - plen`) and a `SIZE_MAX` guard on
  the LCS row sizing.
- Tests pinned to the crate's assertions (banana search/node-count/longest-
  repeated, LCS) plus edge cases, run under GCC and Clang via `iso-harness`.
