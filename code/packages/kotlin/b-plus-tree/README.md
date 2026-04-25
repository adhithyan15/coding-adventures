# B+ Tree (Kotlin) — DT12

A generic, ordered B+ tree implementation in Kotlin with full range-scan support.

This is the idiomatic Kotlin port of the Java DT12 package. The data structure semantics are identical; the code uses Kotlin idioms: sealed classes, `when` expressions, `MutableList`, `require()`, extension functions, and operator overloading via `Comparable`.

## What Is a B+ Tree?

A B+ tree is a height-balanced search tree where:

1. **All data lives at the leaves.** Internal nodes store only *separator keys* for routing — no values. Denser internal nodes → shallower trees → fewer I/Os.

2. **Leaf nodes form a sorted linked list.** Range scans locate the first leaf via the tree (O(log n)), then walk the list sequentially (O(k)), never backtracking.

These properties make B+ trees the standard index structure for relational databases (PostgreSQL, MySQL, SQLite) and file systems (NTFS, ext4, HFS+).

## Usage

```kotlin
import com.codingadventures.bplustree.BPlusTree

val tree = BPlusTree<Int, String>(t = 3)

// Insert / update
tree.insert(10, "ten")
tree.insert(5, "five")
tree.insert(10, "TEN")   // replaces value; size unchanged

// Point lookup
val v = tree.search(10)           // → "TEN"
val present = tree.contains(5)   // → true

// Range scan (inclusive on both ends) — O(log n + k)
val range = tree.rangeScan(5, 15)
// → [(5,"five"), (10,"TEN")]

// Full scan via linked list — O(n)
val all = tree.fullScan()

// Min / max
val lo = tree.minKey()   // → 5
val hi = tree.maxKey()   // → 10

// Metadata
val sz = tree.size       // → 2
val ht = tree.height()   // → 0 for single leaf, 1+ after splits

// Delete
tree.delete(10)          // no-op if absent

// Iterator (ascending key order)
for ((key, value) in tree) println("$key → $value")

// Invariant check (O(n), for testing)
val ok = tree.isValid()
```

## Key Design Choices

### Sealed class hierarchy

```kotlin
private sealed class BPlusNode<K, V>
private class InternalNode<K, V> : BPlusNode<K, V>() { /* keys + children */ }
private class LeafNode<K, V>    : BPlusNode<K, V>() { /* keys + values + next */ }
```

`when` on the sealed class is exhaustive — the Kotlin compiler enforces every case is handled.

### Leaf split vs internal split

```
Leaf split  (B+ tree specific):
  [1,2,|3|,4] → parent gets 3; left=[1,2]; right=[3,4]  (3 stays in right leaf)

Internal split (same as B-tree):
  keys=[k0,k1,|k2|,k3] → parent gets k2; left=[k0,k1]; right=[k3]  (k2 removed)
```

### Routing invariant (not strict equality)

After a non-structural delete the separator may be stale. `isValid()` checks the routing invariant: `max(children[i]) < keys[i]` and `min(children[i+1]) >= keys[i]`. This is the correct invariant for B+ trees.

## Minimum Degree

| `t` | Min keys/node | Max keys/node |
|-----|--------------|---------------|
| 2   | 1            | 3             |
| 3   | 2            | 5             |
| 4   | 3            | 7             |

## Complexity

| Operation | Time |
|-----------|------|
| `search` / `contains` | O(t · log_t n) |
| `insert` | O(t · log_t n) |
| `delete` | O(t · log_t n) |
| `rangeScan(lo, hi)` | O(t · log_t n + k) |
| `fullScan` / `iterator` | O(n) |
| `minKey` | O(1) |
| `maxKey` | O(log_t n) |

## Building

```
gradle test
```

## See Also

- `code/packages/java/b-plus-tree` — Java implementation (same data structure, same tests)
- `code/specs/b-plus-tree.md` — specification
