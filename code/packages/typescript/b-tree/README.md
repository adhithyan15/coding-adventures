# @coding-adventures/b-tree

**DT11** — A generic, self-balancing B-tree for TypeScript.

## What is a B-tree?

A B-tree is a generalization of a binary search tree designed for systems that
read and write large blocks of data. Every node can hold many keys, so the tree
stays very shallow (O(log_t n) levels) and requires far fewer I/O operations
than a BST.

```
          [20 | 50]
         /    |    \
    [10]   [30|40]   [60|70]
```

## Installation

```bash
npm install @coding-adventures/b-tree
```

## Quick start

```typescript
import { BTree } from "@coding-adventures/b-tree";

// Create a B-tree with minimum degree t=2 and a numeric comparator
const tree = new BTree<number, string>(2, (a, b) => a - b);

tree.insert(10, "ten");
tree.insert(5,  "five");
tree.insert(20, "twenty");
tree.insert(15, "fifteen");

console.log(tree.search(10));           // "ten"
console.log(tree.contains(99));         // false
console.log(tree.minKey());             // 5
console.log(tree.maxKey());             // 20
console.log(tree.size);                 // 4
console.log(tree.height());             // 1 (approximately)
console.log(tree.inorder());            // [[5,…], [10,…], [15,…], [20,…]]
console.log(tree.rangeQuery(5, 15));    // [[5,…], [10,…], [15,…]]

tree.delete(10);
console.log(tree.isValid());            // true
```

## API

### `new BTree<K, V>(t, compareFn)`

| Parameter   | Type                        | Default | Description                       |
|-------------|-----------------------------|---------|-----------------------------------|
| `t`         | `number`                    | `2`     | Minimum degree. Must be ≥ 2.      |
| `compareFn` | `(a: K, b: K) => number`    | —       | Returns negative/0/positive.      |

### Methods

| Method                         | Returns             | Description                                |
|-------------------------------|---------------------|--------------------------------------------|
| `insert(key, value)`          | `void`              | Insert or update. O(t·log_t n).            |
| `delete(key)`                 | `boolean`           | Remove key. Returns false if not found.    |
| `search(key)`                 | `V \| undefined`    | Get value by key.                          |
| `contains(key)`               | `boolean`           | True if key exists.                        |
| `minKey()`                    | `K \| undefined`    | Smallest key.                              |
| `maxKey()`                    | `K \| undefined`    | Largest key.                               |
| `rangeQuery(low, high)`       | `Array<[K, V]>`     | Sorted pairs where low ≤ key ≤ high.       |
| `inorder()`                   | `Array<[K, V]>`     | All pairs in sorted order.                 |
| `height()`                    | `number`            | Number of edges on longest root-to-leaf.   |
| `isValid()`                   | `boolean`           | Checks all B-tree invariants.              |
| `size` (getter)               | `number`            | Number of key-value pairs.                 |

## How it fits in the stack

This package implements DT11 in the data structures learning track. It pairs with:

- `@coding-adventures/b-plus-tree` (DT12) — the B+ tree variant used in most databases
- `@coding-adventures/tree` (DT09) — rooted general trees

## Minimum degree guide

| `t` | Min keys/node | Max keys/node | Use when                          |
|-----|---------------|---------------|-----------------------------------|
| 2   | 1             | 3             | Learning / in-memory use          |
| 3   | 2             | 5             | Small page sizes                  |
| 5   | 4             | 9             | Moderate page sizes               |
| 128 | 127           | 255           | 4 KB disk pages (typical DB use)  |

## References

- Cormen, Leiserson, Rivest, Stein — *Introduction to Algorithms*, 4th ed., Chapter 18
- Bayer & McCreight (1972) — *Organization and Maintenance of Large Ordered Indexes*
