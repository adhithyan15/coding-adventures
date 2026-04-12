# Changelog

## 0.1.0 (2026-04-11)

### Added
- Initial release of the Elixir B+ tree package (DT12)
- Immutable, functional B+ tree with values stored only in leaf nodes
- Node representation: `{:leaf, keys, values}` and `{:internal, keys, children}` (no values in internal!)
- Tree representation: `{t, root}` tuple
- `new/0` and `new/1` constructors
- `insert/3` with proactive split strategy; leaf splits COPY separator to parent
  (stays in right leaf), internal splits MOVE separator up (B+ tree rule)
- `delete/3` with borrow-from-right, borrow-from-left, and merge strategies;
  separator keys refreshed after every deletion
- `search/2` always descending to a leaf node
- `member?/2` — predicate wrapper around search
- `range_scan/3` — O(log n + k) range scan
- `full_scan/1` — O(n) full scan via in-order leaf traversal
- `to_list/1` — alias for `full_scan/1`
- `min_key/1` and `max_key/1` (raises ArgumentError on empty tree)
- `size/1`, `empty?/1`, `height/1` accessors
- `valid?/1` checking all B+ tree invariants including separator == min of right child
- Binary search within leaf nodes (divide-and-conquer recursion)
- Routing index for internal nodes (count separators <= key)
- 65+ tests covering all cases, stress tests with 1000+ keys, t=2/3/5
