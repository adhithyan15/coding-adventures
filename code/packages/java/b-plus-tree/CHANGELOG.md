# Changelog — java/b-plus-tree

## [0.1.0] — 2026-04-25

### Added

- **`BPlusTree<K,V>`** — generic B+ tree (DT12) with configurable minimum degree `t ≥ 2`.
  - Inner sealed node hierarchy: `InternalNode<K,V>` (separator keys only) and `LeafNode<K,V>` (key+value pairs + linked-list next pointer).
  - `insert(K, V)` — O(t · log_t n). Duplicate key updates value in-place (size unchanged).
  - `search(K) → V?` — O(t · log_t n). Always descends to a leaf; never terminates at an internal node.
  - `contains(K) → boolean`.
  - `delete(K)` — O(t · log_t n). Fixes underflow via borrow-right, borrow-left, merge-right, merge-left in that preference order.
  - `rangeScan(K, K) → List<Entry>` — O(t · log_t n + k). Locates the starting leaf via tree traversal, then walks the leaf linked list without backtracking.
  - `fullScan() → List<Entry>` — O(n). Walks the entire leaf linked list from `firstLeaf`.
  - `minKey()` — O(1). Reads `firstLeaf.keys[0]`.
  - `maxKey()` — O(log_t n). Follows the rightmost child path.
  - `height()`, `size()`, `isEmpty()`.
  - `iterator()` — ascending key order via `fullScan()`.
  - `isValid()` — O(n) invariant check: leaf depth consistency, key-count bounds, sorted linked list, and routing invariant for separator keys.
  - `toString()` — includes `size`, `height`, and `t`.

- **Leaf split semantics**: separator key is *copied* to the parent AND stays in the right leaf (unlike B-tree where it is *moved* to the parent only).

- **Internal split semantics**: median is *moved* to the parent and removed from both halves (same as B-tree).

- **Routing invariant** (not strict separator equality): after a non-structural delete the separator key in an internal node may be stale (the deleted key was a separator copy). The routing invariant — all keys in `children[i]` < `keys[i]` and all keys in `children[i+1]` ≥ `keys[i]` — is sufficient for correct search behaviour and is what `isValid()` enforces.

- **`BPlusTreeTest`** — 82 JUnit 5 tests:
  - Empty-tree edge cases (size, isEmpty, height, search, contains, fullScan, rangeScan, iterator, minKey, maxKey, delete, isValid).
  - Constructor validation (degree < 2 throws).
  - Single-key operations.
  - In-place update / duplicate insert.
  - Leaf split mechanics: height before/after, separator stays in right leaf, linked-list integrity.
  - Multiple splits in sequential, reverse, and random order.
  - Height monotonicity check over 50 sequential inserts.
  - Range scan: exact bounds, entire tree, no results, single key, absent key, cross-leaf boundary, brute-force exhaustive (all sub-ranges of 1..20).
  - Full scan and iterator vs `fullScan()`.
  - Min/max key after various mutations.
  - Delete: no underflow, absent key (no-op), all three from a leaf.
  - Delete borrow-from-right, borrow-from-left.
  - Delete merge: height decreases, linked-list integrity preserved.
  - Delete all: sequential, reverse, random order — `isValid()` checked after every delete.
  - Routing invariant (separator) after inserts and after deletes.
  - Parameterised over t ∈ {2, 3, 4, 5, 10}: insert + search, delete all.
  - String keys: insert/search and fullScan sorted.
  - Negative keys, `Integer.MIN_VALUE` / `Integer.MAX_VALUE`.
  - Linked-list integrity across 60 random insert/delete operations vs `TreeMap` reference.
  - Stress: 500 random ops vs `TreeMap` (key space 100, t=3).
  - Stress: sequential 1..1000 insert, rangeScan all, delete all even keys (t=4).
  - Repeat insert→delete×10 rounds.
  - `toString` smoke test.

### Design Decisions

- **Stale separators are acceptable.** Real B+ tree implementations (including PostgreSQL) do not update separator keys after non-structural deletes. The routing invariant is weaker than exact-equality but sufficient for correctness.
- **`firstLeaf` pointer never needs update.** Leaf merges always absorb the right sibling into the left, so the leftmost leaf never changes identity.
- **`SplitResult` as a `record`.** Immutable carrier for promoted key + right node avoids mutable state during split propagation.
