# @coding-adventures/fenwick-tree

Binary Indexed Tree (Fenwick Tree) for prefix sums with point updates.

## Quick Start

```ts
import { FenwickTree } from "@coding-adventures/fenwick-tree";

const tree = FenwickTree.fromList([3, 2, 1, 7, 4]);
console.log(tree.prefixSum(3)); // 6
console.log(tree.rangeSum(2, 4)); // 10

tree.update(3, 5);
console.log(tree.pointQuery(3)); // 6
console.log(tree.findKth(11)); // 4
```

## Operations

- `update(i, delta)` in `O(log n)`
- `prefixSum(i)` in `O(log n)`
- `rangeSum(left, right)` in `O(log n)`
- `pointQuery(i)` in `O(log n)`
- `findKth(k)` in `O(log n)` for non-negative frequency arrays
