# Changelog

## 0.1.0 (2026-04-11)

### Added
- Initial release of the Elixir B-tree package (DT11)
- Immutable, functional B-tree following CLRS Chapter 18
- Node representation: `{:leaf, keys, values}` and `{:internal, keys, values, children}`
- Tree representation: `{t, root}` tuple
- `new/0` and `new/1` constructors
- `insert/3` with proactive split strategy and upsert (update existing key)
- `delete/3` with all CLRS cases: leaf (Case 1), internal with predecessor (2a),
  internal with successor (2b), merge (2c), and fill-before-descent with
  rotate-right, rotate-left, and merge (Case 3)
- `search/2` returning `{:ok, value}` or `:error`
- `member?/2` — predicate wrapper around search
- `min_key/1` and `max_key/1` (raises ArgumentError on empty tree)
- `range_query/3` returning sorted `{key, value}` pairs in `[low, high]`
- `inorder/1` returning all `{key, value}` pairs in sorted order
- `size/1`, `empty?/1`, `height/1` accessors
- `valid?/1` checking all B-tree invariants (key counts, sorted order,
  uniform leaf depth, BST property between children and separators)
- Binary search within nodes using divide-and-conquer recursion
- 70+ tests covering all cases, stress tests with 1000+ keys, t=2/3/5
