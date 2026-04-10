# coding-adventures-fenwick-tree-native

Rust-backed Fenwick tree for Python via the repo's zero-dependency `python-bridge`.

## What It Provides

- A native `FenwickTree` class backed by the Rust [fenwick-tree](../../rust/fenwick-tree/) crate
- O(log n) point updates, prefix sums, range sums, point queries, and order-statistic lookup
- Native Python exceptions mirroring the pure package

## Usage

```python
from fenwick_tree_native import FenwickTree

tree = FenwickTree.from_list([3, 2, 1, 7, 4])
assert tree.prefix_sum(3) == 6.0
tree.update(3, 5)
assert tree.point_query(3) == 6.0
assert tree.find_kth(10) == 4
```
