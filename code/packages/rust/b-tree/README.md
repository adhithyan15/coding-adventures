# coding-adventures-b-tree (DT11)

A complete, from-scratch B-tree implementation in Rust.

## What is a B-tree?

A B-tree is a self-balancing search tree generalised from binary search trees.
Invented by Bayer and McCreight at Boeing Research Labs in 1970, it is designed
for systems where data lives on slow block devices (hard drives or SSDs).  The
key insight is to pack *many* keys into one node so that a single disk read
fetches many keys, keeping the number of I/O operations logarithmically small.

```
                    [30 | 60]
                   /    |    \
         [10|20]  [40|50]  [70|80]
         / | \    / | \    / | \
        …  …  …  …  …  …  …  …  …   ← leaves (all at the same depth)
```

### Minimum degree `t`

Every non-root node holds between `t-1` and `2t-1` keys.  The root holds
between 1 and `2t-1` keys.  `t = 2` gives a *2-3-4 tree* (1–3 keys per node).
A database index might use `t = 512` so the tree fits in 3–4 disk pages.

## How it fits in the stack

This crate is part of the `DT` (Data Trees) series:

| ID   | Crate                             | Description                      |
|------|-----------------------------------|----------------------------------|
| DT09 | `coding-adventures-binary-tree`   | Basic binary tree                |
| DT10 | `coding-adventures-avl-tree`      | AVL self-balancing tree          |
| DT11 | **`coding-adventures-b-tree`**    | B-tree (this crate)              |
| DT12 | `coding-adventures-b-plus-tree`   | B+ tree with leaf linked list    |

## Usage

```rust
use coding_adventures_b_tree::BTree;

let mut tree: BTree<i32, &str> = BTree::new(3); // t = 3

tree.insert(10, "ten");
tree.insert(5,  "five");
tree.insert(20, "twenty");
tree.insert(15, "fifteen");

// Point lookup — O(log n)
assert_eq!(tree.search(&10), Some(&"ten"));
assert_eq!(tree.search(&99), None);

// Range query — O(t · log_t n + k)
let results = tree.range_query(&8, &16);
// results: [(&10, &"ten"), (&15, &"fifteen")]

// Sorted iteration
for (key, val) in tree.inorder() {
    println!("{key}: {val}");
}

// Deletion
assert!(tree.delete(&5));
assert!(!tree.contains(&5));

// Structural validation
assert!(tree.is_valid());
```

## Operations

| Method         | Time complexity     | Description                          |
|----------------|---------------------|--------------------------------------|
| `insert`       | O(t · log_t n)      | Proactive top-down splitting          |
| `delete`       | O(t · log_t n)      | CLRS algorithm, all cases            |
| `search`       | O(log n)            | Binary search at each node           |
| `contains`     | O(log n)            | Thin wrapper around `search`         |
| `min_key`      | O(height)           | Leftmost leaf, first key             |
| `max_key`      | O(height)           | Rightmost leaf, last key             |
| `range_query`  | O(t · log_t n + k)  | Prunes subtrees outside range        |
| `inorder`      | O(n)                | Full sorted traversal                |
| `len`          | O(1)                | Cached counter                       |
| `height`       | O(height)           | Recursive descent on first path      |
| `is_valid`     | O(n)                | Full structural check                |

## Implementation notes

- **Proactive top-down splitting**: full nodes are split *before* descending
  into them during insertion.  This avoids backtracking and is cache-friendly.
- **CLRS deletion** (Chapter 18): pre-fills nodes on the way down so every
  node has ≥ t keys before we recurse, enabling in-place deletion without
  upward fixup.
- All code is written in the literate programming style: detailed doc comments
  with ASCII diagrams explain every algorithm and data structure decision.
