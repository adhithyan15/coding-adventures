# fenwick-tree

A Fenwick tree, or Binary Indexed Tree, for prefix sums with point updates in O(log n) time.

## What It Provides

- `FenwickTree` with O(n) construction from a slice
- `update`, `prefix_sum`, `range_sum`, and `point_query`
- `find_kth` for order-statistics style frequency queries
- Clear error variants for invalid indices, invalid ranges, and empty-tree lookups

## Usage

```rust
use fenwick_tree::FenwickTree;

let mut tree = FenwickTree::from_slice(&[3.0, 2.0, 1.0, 7.0, 4.0]);
assert_eq!(tree.prefix_sum(3).unwrap(), 6.0);
assert_eq!(tree.range_sum(2, 4).unwrap(), 10.0);

tree.update(3, 5.0).unwrap();
assert_eq!(tree.point_query(3).unwrap(), 6.0);
assert_eq!(tree.find_kth(11.0).unwrap(), 3);
```

## Building and Testing

```bash
cargo test -p fenwick-tree -- --nocapture
```
