# Changelog

All notable changes to `coding-adventures-radix-tree` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-08

### Added

- `RadixNode` — generic dataclass node with `is_end`, `value`, and `children`
  indexed by first character of each edge label for O(1) child lookup.
- `RadixTree` — generic radix tree class implementing:
  - `insert(key, value)` — four-case insertion with edge splitting (Cases 1–4)
  - `search(key)` — exact-match lookup traversing full edge labels
  - `delete(key)` — deletion with automatic node merging (reverse of split)
  - `starts_with(prefix)` — prefix existence check; handles mid-edge matches
  - `words_with_prefix(prefix)` — DFS collection of all matching keys, sorted
  - `longest_prefix_match(key)` — returns the longest stored key that is a
    prefix of the given input (useful for IP routing, URL dispatch)
  - `__len__` — O(1) key count
  - `__contains__` — exact membership test
  - `__iter__` — yields keys in lexicographic order
  - `to_dict` — exports all (key, value) pairs to a plain dict
  - `__repr__` — developer-friendly string representation
- Literate-programming style docstrings throughout, with ASCII diagrams
  illustrating the four insertion cases, node merging, and tree structure.
- Comprehensive test suite (`tests/test_radix_tree.py`) targeting 95%+
  coverage:
  - `TestRadixTreeInsert` — all four insertion cases, duplicates, empty key
  - `TestRadixTreeSearch` — found/missing, prefix-only, empty tree
  - `TestRadixTreeDelete` — existing, non-existent, merge trigger, all keys
  - `TestRadixTreeStartsWith` — exact, prefix, mid-edge, empty prefix
  - `TestWordsWithPrefix` — no matches, exact, multiple, empty prefix, sorted
  - `TestLongestPrefixMatch` — full/partial/no match, multiple candidates
  - `TestRadixTreeCompression` — verifies node count is O(keys), not O(chars)
  - `TestEdgeSplitting` — exercises all four split cases; regression for
    previously-inserted keys after each split
  - `TestIterAndLen` — len, iter order, contains
  - `TestToDict` — round-trip export
  - `TestRandomProperty` — 100 random keys inserted and verified; half deleted
    and checked; `words_with_prefix("")` vs `list(t)` consistency
- `pyproject.toml` using hatchling, `src/` layout, `coding-adventures-trie`
  dependency, pytest + coverage configuration (fail under 95%).
- `BUILD` and `BUILD_windows` scripts for the repo's Go build tool.
