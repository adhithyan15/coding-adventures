# Changelog — java/b-tree

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-25

### Added

- `BTree<K extends Comparable<K>, V>` — generic B-tree with minimum degree `t`
- `BTree()` — default constructor (`t = 2`, a 2-3-4 tree)
- `BTree(int t)` — constructor with custom minimum degree; throws
  `IllegalArgumentException` for `t < 2`
- `insert(K, V)` — proactive top-down splitting (CLRS B-TREE-INSERT); updates
  value in place if key already exists
- `delete(K)` — CLRS three-case deletion with pre-fill (3a rotate, 3b merge);
  throws `NoSuchElementException` if key absent; tree shrinks when root empties
- `search(K)` — returns value or `null` if absent
- `contains(K)` — membership test
- `minKey()` / `maxKey()` — O(log_t n) min/max; throw on empty tree
- `rangeQuery(K low, K high)` — returns all entries in `[low, high]` in order
- `inorder()` — full in-order traversal as `Iterable<Map.Entry<K,V>>`
- `height()` — number of levels above leaves (0 for leaf-only tree)
- `size()` / `isEmpty()` — cardinality helpers
- `isValid()` — validates all 6 B-tree structural invariants:
  1. key count bounds (t-1 ≤ n ≤ 2t-1 for non-root)
  2. root has ≥ 1 key (unless empty)
  3. keys within each node are strictly increasing
  4. keys respect BST ordering across parent/child boundaries
  5. internal nodes have exactly `n+1` children
  6. all leaves are at the same depth
- `toString()` — summary of `t`, `size`, and `height`
- Inner `Node<K, V>` with `findKeyIndex()` binary search
- `splitChild()`, `insertNonfull()` helpers for insertion
- `deleteRec()`, `ensureMinKeys()`, `mergeChildren()` helpers for deletion
- `collectInorder()` for in-order traversal
- `validate()` for structural invariant checking
- 47 unit tests covering: construction, insertion (ascending, descending,
  shuffled, root-split-forced, duplicate-key update), search, contains,
  deletion (all CLRS cases), minKey/maxKey, rangeQuery, inorder, height,
  isValid, and a 1000-key stress test versus a reference `TreeMap`

### Notes

- `Node` declared as `static final class Node<K extends Comparable<K>, V>` so
  the binary search in `findKeyIndex` can call `compareTo` — the inner class
  carries its own bound independently of the outer class
- All operations are purely in-memory; no I/O
