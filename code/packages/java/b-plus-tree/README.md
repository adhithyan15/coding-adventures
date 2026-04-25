# B+ Tree (Java) — DT12

A generic, ordered B+ tree implementation in Java 21 with full range-scan support.

## What Is a B+ Tree?

A B+ tree is a height-balanced search tree where:

1. **All data lives at the leaves.** Internal nodes store only *separator keys* for routing — no values. This keeps internal nodes small, allowing higher branching factors and shallower trees.

2. **Leaf nodes form a sorted linked list.** Once you locate the starting leaf for a range query, you walk the list without backtracking up the tree.

These two properties make B+ trees the standard index structure for relational databases (PostgreSQL, MySQL, SQLite) and file systems (NTFS, ext4, HFS+).

## B-Tree vs B+ Tree

```
B-TREE (DT11) — values everywhere:

           [20:"Carol",  40:"Eve"]
          /             |            \
[5:"Alice", 10:"Bob"]  [25:"Dave"]  [45:"Frank", 55:"Grace"]

B+ TREE (DT12) — values only in leaves:

                [20,       40]       ← routing keys only
              /    |          \
    leaf1      leaf2       leaf3
    ↓            ↓            ↓
[(5,A),(10,B),(20,C)] → [(25,D),(40,E)] → [(45,F),(55,G)] → null
```

Key observation: key 20 appears **both** in the internal node **and** in leaf1. This separator copy stays in the leaf — in a B-tree it would only appear in the internal node.

## Leaf Split vs Internal Split

This is the critical difference from B-tree:

```
B-tree leaf split:   [1, 2, |3|, 4, 5]  →  parent gets 3
                                             left=[1,2]; right=[4,5]   (3 removed!)

B+ tree leaf split:  [1, 2, |3|, 4, 5]  →  parent gets 3
                                             left=[1,2]; right=[3,4,5] (3 stays!)
```

Internal node splits work the same way in both B-tree and B+ tree: the median is *moved* to the parent (not duplicated).

## Why B+ Trees Win in Databases

| Property | B-tree | B+ tree |
|---|---|---|
| Internal node size | key + value | key only |
| Branching factor (4 KiB page) | ~37 | ~500 |
| Tree height for 1B keys | log₃₇(1B) ≈ 6 | log₅₀₀(1B) ≈ 4 |
| Range scan | Tree backtracking | Sequential linked-list walk |

## Usage

```java
import com.codingadventures.bplustree.BPlusTree;

// Default minimum degree t=2
BPlusTree<Integer, String> tree = new BPlusTree<>(3);  // t=3 for wider nodes

// Insert / update
tree.insert(10, "ten");
tree.insert(20, "twenty");
tree.insert(5, "five");
// Duplicate key: value is replaced, size unchanged
tree.insert(10, "TEN");

// Point lookup
String v = tree.search(10);   // → "TEN"
boolean present = tree.contains(5);  // → true

// Range scan (inclusive on both ends)  — O(log n + k), k = results
List<Map.Entry<Integer,String>> range = tree.rangeScan(5, 15);
// → [(5,"five"), (10,"TEN")]

// Full scan via linked list  — O(n)
List<Map.Entry<Integer,String>> all = tree.fullScan();

// Min / max  — O(1) and O(log n) respectively
int lo = tree.minKey();  // → 5
int hi = tree.maxKey();  // → 20

// Metadata
int sz = tree.size();   // → 3
int ht = tree.height(); // → 0 for single leaf, 1+ after splits

// Delete
tree.delete(10);        // no-op if absent

// Iterator (ascending key order)
for (Map.Entry<Integer,String> e : tree) { ... }

// Invariant check (O(n), for testing)
boolean ok = tree.isValid();
```

## Minimum Degree

The minimum degree `t` controls the node capacity:

| `t` | Min keys/node | Max keys/node | Max children |
|-----|--------------|---------------|--------------|
| 2   | 1            | 3             | 4            |
| 3   | 2            | 5             | 6            |
| 4   | 3            | 7             | 8            |

Higher `t` → wider nodes, shallower tree, better cache performance. Use `t=2` for testing; use `t=128` or higher for on-disk workloads.

## Complexity

| Operation | Time |
|-----------|------|
| `search`  | O(t · log_t n) |
| `insert`  | O(t · log_t n) |
| `delete`  | O(t · log_t n) |
| `rangeScan(lo, hi)` | O(t · log_t n + k) |
| `fullScan` | O(n) |
| `minKey`  | O(1) |
| `maxKey`  | O(log_t n) |

## Package Structure

```
src/
  main/java/com/codingadventures/bplustree/
    BPlusTree.java          ← single-file implementation + literate comments
  test/java/com/codingadventures/bplustree/
    BPlusTreeTest.java      ← 82 tests: empty, split, merge, stress, isValid
```

## Building

```
gradle test
```

## Relation to Other DT Packages

| Package | DT | Description |
|---------|----|-------------|
| `binary-search-tree` | DT01 | Unbalanced BST — worst case O(n) |
| `binary-tree` | DT04 | Generic binary tree |
| `fenwick-tree` | DT06 | BIT for prefix sums |
| `segment-tree` | DT05 | Range queries with monoids |
| `b-plus-tree` | DT12 | **This package** — ordered index, O(log n) + linked-list scans |
