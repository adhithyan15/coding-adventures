# Changelog — trie (Kotlin)

## [0.1.0] — 2026-04-25

### Added
- Initial implementation of a generic prefix tree (Trie) mapping `String`
  keys to values of type `V`, as an idiomatic Kotlin `class`.
- HashMap-based inner `Node` using `getOrPut` for ergonomic child creation.
- `insert(key, value?)` — supports nullable values; increments `size` only
  on first insertion of a key.
- `operator fun get(key)` — Kotlin index-style lookup returning `V?`.
- `contains(key)` — true only for inserted complete keys.
- `startsWith(prefix)` — true if any inserted key begins with prefix.
- `delete(key)` — removes end-marker; shared nodes preserved.
- `keysWithPrefix(prefix)` — depth-first enumeration of matching keys.
- `keys()` — all keys.
- `size: Int` and `isEmpty: Boolean` — O(1) Kotlin properties.
- Literate source with diagram and complexity table.
- 34 unit tests covering all operations including Unicode keys and a
  1000-element smoke test.
