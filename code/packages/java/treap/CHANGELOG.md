# Changelog — java/treap

## [0.1.0] — 2026-04-25

### Added
- `Treap` — purely functional treap using Java 21 records for Node
- `Node` record — immutable node with key, priority, left, right
- `SplitResult` record — returned by `split()` as a named pair
- `Treap.Builder` — fluent builder for constructing treaps from existing nodes
- `Treap.empty()` / `Treap.withSeed(long)` / `Treap.empty(Random)` factories
- `splitNode(Node, int)` — inclusive split (≤ key, > key)
- `splitStrict(Node, int)` — strict split (< key, ≥ key) used by delete
- `mergeNodes(Node, Node)` — merge two subtrees by priority
- `Treap.merge(Treap, Treap)` — public static merge of two treaps
- `insert(int)` — random priority assignment via seeded RNG
- `insertWithPriority(int, double)` — deterministic priority for testing
- `delete(int)` — via two splits + merge
- `split(int)` — public API, returns SplitResult
- `contains(int)` — iterative BST search
- `min()` / `max()` — Optional-returning extremes
- `predecessor(int)` / `successor(int)` — Optional floor/ceiling
- `kthSmallest(int)` — 1-indexed via in-order traversal
- `toSortedList()` — ascending in-order traversal
- `isValidTreap()` — verifies BST and heap properties
- `size()`, `height()`, `isEmpty()` — structural metrics
- 44 unit tests covering: empty, single element, deterministic priorities,
  ascending/descending/mixed inserts, duplicates, split, merge, delete
  (leaf/root/all), round-trip, immutability, random stress (200 inserts,
  100 deletes)
