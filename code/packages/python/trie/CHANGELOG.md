# Changelog

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
