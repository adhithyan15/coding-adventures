# Changelog

## 0.1.0 (2026-04-11)

### Added
- Initial release of the `coding_adventures_b_tree` gem (DT11)
- `BTree` class with configurable minimum degree `t` (default t=2)
- `insert(key, value)` with proactive split strategy (CLRS Chapter 18)
- `delete(key)` with all sub-cases: leaf removal (Case 1), internal node
  with predecessor/successor replacement (Cases 2a/2b), merge (Case 2c),
  rotate-left/rotate-right borrow, and child fill-up before descent (Case 3)
- `search(key)` returning the value or nil
- `include?(key)` / `member?(key)` aliases
- `[](key)` raising `KeyError` if absent; `[]=(key, value)` for upsert
- `min_key` / `max_key` walking the leftmost/rightmost leaf path
- `range_query(low, high)` returning sorted `[[key, value], ...]` pairs
- `inorder` returning all pairs in sorted order
- `size`, `empty?`, `height` accessors
- `valid?` checking all B-tree invariants (key counts, sorted order, child
  counts, uniform leaf depth, BST property)
- `BTreeNode` with binary search (`search`) for O(log(node_size)) within-node lookup
- 80+ tests covering all delete cases, stress tests with 1000+ keys, t=2/3/5
