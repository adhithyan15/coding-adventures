# Changelog

## 0.1.0 (2026-04-11)

### Added
- Initial release of the `coding_adventures_b_plus_tree` gem (DT12)
- `BPlusTree` class with configurable minimum degree `t` (default t=2)
- Two node types: `BPlusLeafNode` (stores key-value pairs + next_leaf pointer)
  and `BPlusInternalNode` (stores separator keys + child pointers only)
- `insert(key, value)` with proactive splits; leaf splits COPY the separator
  key to parent while keeping it in the right leaf (B+ tree rule)
- `delete(key)` with borrow-from-left, borrow-from-right, and merge strategies;
  leaf linked list correctly maintained after all merges
- `search(key)` always reaching a leaf node
- `include?(key)` / `member?(key)` aliases
- `[](key)` raising `KeyError`; `[]=(key, value)` for upsert
- `range_scan(low, high)` walking the leaf linked list — O(log n + k), no backtracking
- `full_scan` walking the entire leaf linked list — O(n)
- `each` block for Enumerable support (map, select, min_by, etc.)
- `min_key` / `max_key` via leftmost/rightmost leaf
- `size`, `empty?`, `height` accessors
- `valid?` checking all B+ tree invariants plus leaf linked list integrity
- 90+ tests covering all cases, stress tests with 1000+ keys, t=2/3/5
