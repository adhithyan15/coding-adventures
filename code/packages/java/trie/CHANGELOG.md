# Changelog — trie (Java)

## [0.1.0] — 2026-04-25

### Added
- Initial implementation of a generic prefix tree (Trie) mapping `String`
  keys to values of type `V`.
- HashMap-based node design: each node holds a `Map<Character, Node>` for
  children, a boolean `isEnd`, and a nullable value. This generalises to any
  character set (Unicode, DNA bases, etc.) unlike a fixed 26-slot array.
- `insert(key, value)` — O(len(key)). If the key already exists, updates the
  value and leaves `size` unchanged.
- `get(key)` — O(len(key)). Returns `Optional<V>`; empty if key not present.
- `contains(key)` — O(len(key)). True only for inserted complete keys.
- `startsWith(prefix)` — O(len(prefix)). True if any key begins with prefix.
- `delete(key)` — O(len(key)). Removes only the end-marker; shared prefixes
  survive deletion.
- `keysWithPrefix(prefix)` — returns all keys with the given prefix via
  depth-first traversal.
- `keys()` — all keys (equivalent to `keysWithPrefix("")`).
- `size()`, `isEmpty()` — O(1) via a maintained counter.
- `insert(null, ...)` throws `IllegalArgumentException`.
- Literate source with trie diagram and complexity table.
- 34 unit tests covering insert/get, contains, startsWith, delete, prefix
  queries, size, empty key, Unicode, and large dataset smoke test.
