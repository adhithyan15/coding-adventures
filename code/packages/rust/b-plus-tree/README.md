# coding-adventures-b-plus-tree (DT12)

A complete, from-scratch B+ tree implementation in Rust with a leaf linked
list for O(k) range scans.

## What is a B+ tree?

A B+ tree is a variant of the B-tree that stores all values exclusively in
leaf nodes.  Internal nodes hold only separator keys for routing.  Leaves are
linked together in a sorted singly-linked list, enabling efficient sequential
scans without any tree traversal.

```
             [30 | 60]            ← internal node (separator keys only)
            /    |    \
     [10|20]  [40|50]  [70|80]   ← internal nodes
     / | \    / | \    / | \
 L1  L2  L3  L4  L5  L6  L7    ← leaf nodes with values
 └──►L2──►L3──►L4──►L5──►L6──►L7──►∅   (linked list)
```

### B-tree vs B+ tree at a glance

| Feature              | B-tree                   | B+ tree                     |
|---------------------|--------------------------|-----------------------------|
| Values stored        | At every node            | Only in leaf nodes           |
| Internal node size   | Keys + values + children | Keys + children (smaller)   |
| Range query          | Complex in-order walk    | Walk leaf list from start   |
| Full scan            | O(n) with recursion      | O(n) list walk, no recursion|

## How it fits in the stack

| ID   | Crate                             | Description                      |
|------|-----------------------------------|----------------------------------|
| DT11 | `coding-adventures-b-tree`        | B-tree                           |
| DT12 | **`coding-adventures-b-plus-tree`** | B+ tree (this crate)           |
| DT13 | `coding-adventures-lsm-tree`      | LSM tree (planned)               |

## Usage

```rust
use coding_adventures_b_plus_tree::BPlusTree;

let mut tree: BPlusTree<i32, &str> = BPlusTree::new(3);

tree.insert(10, "ten");
tree.insert(5,  "five");
tree.insert(20, "twenty");
tree.insert(15, "fifteen");

// Point lookup
assert_eq!(tree.search(&10), Some(&"ten"));

// Range scan — walks leaf linked list after finding start point
let results = tree.range_scan(&8, &16);
// results: [(&10, &"ten"), (&15, &"fifteen")]

// Full scan — O(n) list walk, no tree traversal
for (key, val) in tree.full_scan() {
    println!("{key}: {val}");
}

// Iterator (reference)
for (key, val) in tree.iter() { /* … */ }

// Consuming iterator
for (key, val) in tree { /* … */ }

// Deletion
assert!(tree.delete(&5));

// Structural validation
assert!(tree.is_valid());
```

## Operations

| Method       | Time              | Description                              |
|-------------|-------------------|------------------------------------------|
| `insert`    | O(t · log_t n)    | Leaf split copies separator to parent    |
| `delete`    | O(t · log_t n)    | Bottom-up rebalance, list pointers fixed |
| `search`    | O(log n)          | Find leaf, binary search within it       |
| `contains`  | O(log n)          | Wrapper around `search`                  |
| `range_scan`| O(log n + k)      | Walk leaf list from first matching leaf  |
| `full_scan` | O(n)              | Walk entire leaf list from `first_leaf`  |
| `min_key`   | O(1) amortised    | Via `first_leaf` pointer                 |
| `max_key`   | O(height)         | Rightmost leaf                           |
| `len`       | O(1)              | Cached counter                           |
| `height`    | O(height)         | Recursive descent                        |
| `is_valid`  | O(n)              | Full structural + list check             |

## Safety

The `BPlusLeaf::next` field is a `*mut BPlusLeaf<K, V>` raw pointer forming
the leaf linked list.  The invariant:

> Every leaf is owned transitively by `BPlusTree::root` through `Box`.  The
> tree never exposes raw pointers to callers.  Mutations only happen through
> `&mut BPlusTree`, ensuring exclusive access.  Therefore all raw pointer
> dereferences inside the tree's methods are sound.

This is a well-known Rust pattern for intrusive linked structures where one
owner keeps all nodes alive.

## Implementation notes

- Leaf split: the separator promoted to the parent is *copied* from the first
  key of the right leaf — the right leaf retains the key, matching B+ tree
  semantics.
- `first_leaf` is updated on every insert/delete so full-scan and min_key
  are always O(1) to start.
- `is_valid` independently verifies both the tree structure and the leaf
  linked list, including a count match against `size`.
