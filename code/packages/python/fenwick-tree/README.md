# coding-adventures-fenwick-tree

Binary Indexed Tree (Fenwick Tree) for prefix sums with point updates.

## What It Is

A Fenwick tree (invented by Peter Fenwick, 1994) answers two queries in O(log n) time
using O(n) space:

- **Prefix sum**: sum of elements from index 1 to i
- **Point update**: add delta to element at index i

It achieves this through a single bit trick: `lowbit(i) = i & (-i)`, which extracts the
lowest set bit of i. Each cell `bit[i]` stores the sum of `lowbit(i)` consecutive
elements ending at position i.

## When to Use

Use a Fenwick tree when you need both prefix/range sums and point updates and:
- You don't need range updates (use a Segment Tree for those)
- You want minimal code and maximum speed
- Your combine function is invertible (sum, product, XOR — not min/max)

## Installation

```bash
pip install coding-adventures-fenwick-tree
```

## Usage

```python
from fenwick_tree import FenwickTree

# Build from a list (O(n))
ft = FenwickTree.from_list([3, 2, 1, 7, 4])

# Prefix sum: sum of positions 1..i (1-indexed)
ft.prefix_sum(3)    # → 6   (3+2+1)
ft.prefix_sum(5)    # → 17  (3+2+1+7+4)

# Range sum: sum of positions l..r
ft.range_sum(2, 4)  # → 10  (2+1+7)

# Point query: value at position i
ft.point_query(4)   # → 7

# Point update: add delta to position i
ft.update(3, 5)     # arr[3] is now 6
ft.prefix_sum(3)    # → 11  (3+2+6)

# Order statistics: find smallest i where prefix_sum(i) >= k
arr = [1, 2, 3, 4, 5]  # prefix sums: 1, 3, 6, 10, 15
ft2 = FenwickTree.from_list(arr)
ft2.find_kth(4)     # → 3  (prefix_sum(3) = 6 is first >= 4)

# Size
len(ft)             # → 5
```

## API

| Method | Time | Description |
|--------|------|-------------|
| `FenwickTree(n)` | O(n) | Create empty tree of size n |
| `FenwickTree.from_list(values)` | O(n) | Build from list |
| `update(i, delta)` | O(log n) | Add delta to position i (1-indexed) |
| `prefix_sum(i)` | O(log n) | Sum of positions 1..i |
| `range_sum(l, r)` | O(log n) | Sum of positions l..r |
| `point_query(i)` | O(log n) | Value at position i |
| `find_kth(k)` | O(log n) | Smallest i where prefix_sum(i) >= k |
| `len(ft)` | O(1) | Number of positions |

## How It Works

The algorithm hinges on `lowbit(i) = i & (-i)`:

- **Query**: start at i, add `bit[i]`, jump to `i - lowbit(i)`, repeat until 0
- **Update**: start at i, add delta to `bit[i]`, jump to `i + lowbit(i)`, repeat while <= n
- **Build**: O(n) by propagating each cell to its parent `i + lowbit(i)` exactly once

## Layer Position (DT Series)

```
DT04: heap
DT05: segment-tree   ← more general sibling
DT06: fenwick-tree   ← [THIS PACKAGE]
```

## Running Tests

```bash
uv run python -m pytest tests/ -v
```
