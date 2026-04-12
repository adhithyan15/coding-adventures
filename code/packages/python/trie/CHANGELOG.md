# Changelog

## [0.2.0] — 2026-04-11

### Added

- `TrieCursor[K, V]` — a generic step-by-step trie cursor for streaming algorithms.
  - `step(element: K) -> bool` — advance along a child edge; returns False if missing.
  - `insert(element: K, value: V) -> None` — add a child at the current position.
  - `reset() -> None` — return to root.
  - `value: V | None` — value at current node.
  - `at_root: bool` — whether cursor is at the trie root.
  - `__iter__` — iterate all `([key_elements], value)` pairs (DFS order).
  - `__len__` and `__bool__` for size queries.
  - Generic key type `K: Hashable` — supports `int` (bytes), `str` (chars), or any hashable.
- 20 new `TrieCursor` tests, including an LZ78 encoding simulation.
- All 95 tests pass, 99.4% coverage.

## [0.1.0] — 2026-04-08

### Added

- `Trie[V]` generic class implementing the prefix tree data structure
- `insert(key, value)` for O(k) key insertion (creates new word or updates value)
- `search(key)` for O(k) exact-match lookup returning value or None
- `delete(key)` for O(k) key deletion with node pruning; returns False if not found
- `starts_with(prefix)` for O(p) prefix existence check
- `words_with_prefix(prefix)` for autocomplete — all keys with prefix in lexicographic order
- `longest_prefix_match(string)` for IP routing / URL dispatch — longest stored key that is a prefix of string
- `all_words()` for O(n·k) full iteration in lexicographic order
- Dict-like interface: `__getitem__`, `__setitem__`, `__delitem__`, `__contains__`
- Iteration support: `__iter__` and `items()` both yield in lexicographic order
- `__len__` for O(1) word count, `__bool__` for emptiness check
- `is_valid()` structural invariant verifier for testing
- `_TrieNode` internal dataclass with dict-based children (supports any character set)
- `TrieError` base exception class
- `KeyNotFoundError` for `__getitem__` and `__delitem__` when key is absent
- Full type annotations including `Generic[V]` for value type
- 95%+ test coverage with all 15 spec test cases implemented
- Literate programming style inline comments explaining every algorithm
