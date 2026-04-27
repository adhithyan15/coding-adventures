# fenwick-tree

Fenwick tree, also called a Binary Indexed Tree, for prefix sums, range sums,
and point updates in `O(log n)` time.

## Layer 1

This package is part of Layer 1 of the coding-adventures computing stack.

## What It Includes

- `FenwickTree(length)` for zero-filled construction
- `FromList` for linear-time tree building from existing values
- `Update`, `PrefixSum`, `RangeSum`, and `PointQuery`
- `FindKth` for prefix-sum order statistics
- Literate comments explaining why `lowbit(index)` reveals each node's range

## Example

```csharp
using CodingAdventures.FenwickTree;

var tree = FenwickTree.FromList([3.0, 2.0, 1.0, 7.0, 4.0]);

Console.WriteLine(tree.PrefixSum(4)); // 13
tree.Update(3, 5.0);
Console.WriteLine(tree.PointQuery(3)); // 6
```

## Development

```bash
# Run tests
bash BUILD
```
