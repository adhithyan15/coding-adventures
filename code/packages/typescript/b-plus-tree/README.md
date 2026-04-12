# @coding-adventures/b-plus-tree

**DT12** — A generic B+ tree for TypeScript, optimized for range scans and full sequential scans.

## What is a B+ tree?

A B+ tree is a variant of the B-tree where:

1. **All data lives in the leaves** — internal nodes hold only routing/separator keys.
2. **Leaves form a linked list** — each leaf has a `next` pointer to the next leaf in sorted order.

This design powers nearly every relational database index you'll encounter (PostgreSQL, MySQL, SQLite, Oracle, SQL Server, InnoDB, etc.).

```
Internal nodes (routing only):
         [20 | 40]
        /    |    \

Leaf nodes (data + linked list):
  [10|15] → [20|25|30] → [40|45|50] → null
```

## Why B+ tree over B-tree?

| Feature               | B-tree                | B+ tree                          |
|-----------------------|-----------------------|----------------------------------|
| Data location         | Every node            | Leaves only                      |
| Full scan             | Complex in-order walk | Walk the leaf linked list O(n)   |
| Range scan            | Subtree traversal     | Binary search + walk leaves O(m) |
| Internal node size    | Larger (holds values) | Smaller (keys only, more entries)|
| Database preference   | Rare                  | Universal                        |

## Installation

```bash
npm install @coding-adventures/b-plus-tree
```

## Quick start

```typescript
import { BPlusTree } from "@coding-adventures/b-plus-tree";

const tree = new BPlusTree<number, string>(2, (a, b) => a - b);

tree.insert(10, "ten");
tree.insert(5,  "five");
tree.insert(20, "twenty");
tree.insert(15, "fifteen");

console.log(tree.search(10));           // "ten"
console.log(tree.contains(99));         // false
console.log(tree.minKey());             // 5  — O(1) via firstLeaf
console.log(tree.maxKey());             // 20
console.log(tree.size);                 // 4
console.log(tree.height());             // 1 (approximately)

console.log(tree.fullScan());           // [[5,…], [10,…], [15,…], [20,…]]
console.log(tree.rangeScan(5, 15));     // [[5,…], [10,…], [15,…]]

// Sorted iteration via Symbol.iterator
for (const [key, value] of tree) {
  console.log(key, value);
}

tree.delete(10);
console.log(tree.isValid());            // true
```

## API

### `new BPlusTree<K, V>(t, compareFn)`

| Parameter   | Type                        | Default | Description                       |
|-------------|-----------------------------|---------|-----------------------------------|
| `t`         | `number`                    | `2`     | Minimum degree. Must be ≥ 2.      |
| `compareFn` | `(a: K, b: K) => number`    | —       | Returns negative/0/positive.      |

### Methods

| Method                         | Returns             | Description                                          |
|-------------------------------|---------------------|------------------------------------------------------|
| `insert(key, value)`          | `void`              | Insert or update. O(t·log_t n).                      |
| `delete(key)`                 | `boolean`           | Remove key. Returns false if not found.              |
| `search(key)`                 | `V \| undefined`    | Get value by key.                                    |
| `contains(key)`               | `boolean`           | True if key exists.                                  |
| `minKey()`                    | `K \| undefined`    | Smallest key. O(1).                                  |
| `maxKey()`                    | `K \| undefined`    | Largest key. O(h).                                   |
| `rangeScan(low, high)`        | `Array<[K, V]>`     | Sorted pairs where low ≤ key ≤ high. O(log_t n + m). |
| `fullScan()`                  | `Array<[K, V]>`     | All pairs in sorted order. O(n).                     |
| `height()`                    | `number`            | Number of edges on longest root-to-leaf path.        |
| `isValid()`                   | `boolean`           | Checks all B+ tree invariants.                       |
| `[Symbol.iterator]()`         | `Iterator<[K, V]>`  | Enables `for...of` and spread.                       |
| `size` (getter)               | `number`            | Number of key-value pairs.                           |

## Key insight: separator COPIED on leaf split

When a leaf is full and needs to split, the separator key that goes up to the
parent is **copied** (not moved). The right leaf keeps the separator key:

```
Before split (t=2, leaf full with 3 keys):
  left leaf = [10, 20, 30]

After split:
  left leaf  = [10]
  right leaf = [20, 30]   ← 20 stays here
  separator 20 COPIED up to parent
```

This is different from a B-tree leaf split (where the median is removed).

## How it fits in the stack

- `@coding-adventures/b-tree` (DT11) — the classic B-tree variant
- `@coding-adventures/b-plus-tree` (DT12) — this package (database standard)

## References

- Cormen, Leiserson, Rivest, Stein — *Introduction to Algorithms*, 4th ed., Chapter 18
- Ramakrishnan & Gehrke — *Database Management Systems*, Chapter 10
- PostgreSQL btree documentation
