# swift/b-tree — B-Tree (DT11)

A generic B-Tree implementation in Swift that maps keys of any `Comparable` type to values.

## What is a B-Tree?

A B-Tree is a self-balancing search tree where each node can hold many keys.  It was
invented at Boeing Research Labs in 1970 and is the data structure powering virtually every
database (PostgreSQL, MySQL, SQLite) and filesystem (NTFS, HFS+, ext4) you will ever use.

The key insight: by letting nodes be "wide" (holding many keys), the tree stays very shallow
even with billions of entries.  A B-Tree with one billion entries and t=500 has a height of
at most 4.

## API

```swift
// Create a B-Tree with minimum degree t (default 2).
let tree = BTree<Int, String>(t: 2)

// Insert / update.
tree.insert(10, "ten")
tree.insert(20, "twenty")
tree.insert(10, "TEN")   // upsert — updates existing key

// Search.
tree.search(10)           // → "TEN"
tree.contains(99)         // → false

// Min / max.
tree.minKey()             // → 10
tree.maxKey()             // → 20

// Traversal.
tree.inorder()            // → [(10, "TEN"), (20, "twenty")]
tree.rangeQuery(from: 5, to: 15)  // → [(10, "TEN")]

// Delete.
tree.delete(10)           // → true
tree.delete(99)           // → false

// Metadata.
tree.count                // → 1
tree.height               // → 0 (single leaf)
tree.isValid()            // → true
```

## Parameters

| Parameter | Meaning |
|-----------|---------|
| `t` | Minimum degree (≥ 2). Nodes hold between `t-1` and `2t-1` keys. |

Larger `t` → shorter, wider tree.  Good for disk-based storage where reading an entire
node in one I/O is cheap.

## Performance

| Operation | Time |
|-----------|------|
| Search | O(t · log_t n) |
| Insert | O(t · log_t n) |
| Delete | O(t · log_t n) |
| In-order scan | O(n) |
| Range query | O(t · log_t n + k) where k = results |

## Stack position

This package is a standalone data structure (DT11) that has no dependencies.
The B+ Tree variant (DT12) lives in `swift/b-plus-tree`.
