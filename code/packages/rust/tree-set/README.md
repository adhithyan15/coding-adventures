# tree_set

DT-style ordered set built on top of the balanced BST crates.

This crate exposes a reusable ordered-set abstraction with a pluggable
backend:

- `TreeSet<T>` defaults to the AVL-backed implementation
- `TreeSet<T, RBTree<T>>` uses the red-black backend explicitly
- both backends support sorted iteration, lookup, deletion, range queries,
  rank, predecessor / successor, and set algebra

## Usage

```rust
use tree_set::{RedBlackTreeSet, TreeSet};

let a = TreeSet::from_list([5, 1, 3, 3, 9]);
assert_eq!(a.to_sorted_array(), vec![1, 3, 5, 9]);
assert_eq!(a.rank(&5), 2);
assert_eq!(a.range(&3, &9, true), vec![3, 5, 9]);

let b = RedBlackTreeSet::from_list([3, 4, 5, 8]);
assert!(b.contains(&4));
assert_eq!(b.intersection(&RedBlackTreeSet::from_list([1, 4, 8])).to_sorted_array(), vec![4, 8]);
```

