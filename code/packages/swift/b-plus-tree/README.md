# swift/b-plus-tree — B+ Tree (DT12)

A generic B+ Tree implementation in Swift.  Like the B-Tree (DT11) but with one key
twist: all values live exclusively in the leaf nodes, and the leaves are connected by a
linked list for fast range and full scans.

## Why B+ Tree over B-Tree?

Use a B+ Tree when you need:
- **Range queries** — "give me all rows where age BETWEEN 25 AND 35"
- **Full table scans** — "scan every record in order"

Use a B-Tree when you need:
- Fast **point lookups** without caring about range scans
- Values associated with internal (non-leaf) nodes

Almost every SQL database (PostgreSQL, MySQL, SQLite) uses B+ Trees for indexes,
not plain B-Trees, precisely because range queries are so common.

## API

```swift
let tree = BPlusTree<Int, String>(t: 2)

// Insert / update.
tree.insert(10, "ten")
tree.insert(20, "twenty")

// Point search.
tree.search(10)           // → "ten"
tree.contains(99)         // → false

// Min / max (reads from firstLeaf / last leaf — O(n/branching)).
tree.minKey()             // → 10
tree.maxKey()             // → 20

// Range scan via linked list — extremely fast.
tree.rangeScan(from: 5, to: 15)   // → [(10, "ten")]

// Full scan walks every leaf via linked list — O(n), no tree traversal.
tree.fullScan()           // → [(10, "ten"), (20, "twenty")]

// In-order (same as fullScan for B+ Trees).
tree.inorder()

// Delete.
tree.delete(10)           // → true

// Metadata.
tree.count                // → 1
tree.height               // → 0
tree.isValid()            // → true
```

## Stack position

Standalone data structure (DT12).  No dependencies.
The plain B-Tree lives at `swift/b-tree` (DT11).
