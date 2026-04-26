# Changelog — kotlin/b-plus-tree

## [0.1.0] — 2026-04-25

### Added

- **`BPlusTree<K : Comparable<K>, V>`** — idiomatic Kotlin port of the Java DT12 B+ tree.
  - Sealed class hierarchy: private `BPlusNode<K,V>` sealed base; `InternalNode<K,V>` (separator keys only, mutable lists); `LeafNode<K,V>` (key+value lists + `next` pointer).
  - `insert(K, V)` — O(t · log_t n). Duplicate key replaces value in-place.
  - `search(K): V?` — always descends to a leaf.
  - `contains(K): Boolean`.
  - `delete(K)` — fixes underflow via borrow-right, borrow-left, merge-right, merge-left.
  - `rangeScan(K, K): List<Map.Entry<K,V>>` — O(t · log_t n + k) using leaf linked list.
  - `fullScan(): List<Map.Entry<K,V>>` — O(n), walks linked list from `firstLeaf`.
  - `minKey(): K` — O(1); throws `NoSuchElementException` if empty.
  - `maxKey(): K` — O(log_t n); throws `NoSuchElementException` if empty.
  - `height(): Int`, `size: Int` (property), `isEmpty: Boolean` (property).
  - `iterator()` — ascending order; satisfies `Iterable<Map.Entry<K,V>>`.
  - `isValid(): Boolean` — O(n) invariant check using routing invariant (not strict separator equality).
  - `toString()` — `BPlusTree{size=N, height=H, t=T}`.
  - `init { require(t >= 2) { ... } }` for constructor validation.

- **`BPlusTreeTest`** — 82 JUnit 5 tests mirroring the Java suite:
  - All empty-tree edge cases (including `assertFailsWith<NoSuchElementException>`).
  - Constructor validation with `assertFailsWith<IllegalArgumentException>`.
  - Single-key CRUD.
  - Duplicate-key insert (value replacement, size unchanged).
  - Leaf split mechanics (height before/after, separator in right leaf, linked-list intact).
  - Multiple splits: sequential, reverse, random order.
  - Height monotonicity over 50 sequential inserts.
  - Range scan: brute-force exhaustive over all sub-ranges of 1..20.
  - Full scan and `toList()` via iterator.
  - Min/max after mutations.
  - Delete: no-underflow, absent key, borrow-right, borrow-left, merge (height decrease, linked-list after merge).
  - Delete all: sequential, reverse, random — `isValid()` after every step.
  - Routing invariant after inserts and deletes (stale separator case).
  - Parameterised over t ∈ {2, 3, 4, 5, 10}: insert+search, delete-all.
  - String keys: insert/search and fullScan sorted.
  - Negative keys, `Int.MIN_VALUE` / `Int.MAX_VALUE`.
  - Linked-list integrity: 60 random insert/delete steps vs `TreeMap` reference.
  - Stress: 500 random ops vs `TreeMap` reference (key space 100, t=3).
  - Stress: sequential 1..1000 insert, rangeScan all, delete all even keys (t=4).
  - Repeat insert→delete × 10 rounds.
  - `toString` smoke test.

### Kotlin Idioms Used

- `sealed class` for exhaustive `when` dispatch on node types.
- `data class SplitResult` for immutable split carrier.
- `MutableList<K>` / `MutableList<V>` instead of `java.util.ArrayList`.
- `require(t >= 2)` for precondition checking.
- `var leaf: LeafNode<K, V>? = firstLeaf` — nullable traversal with `?.next`.
- `Comparable<K>` operators (`>`, `>=`, `<`) instead of `compareTo()` calls.
- `ushr(1)` for unsigned right shift in binary search.
- `java.util.AbstractMap.SimpleImmutableEntry` reused for `Map.Entry` values.
