# fenwick-tree

Fenwick tree, also called a Binary Indexed Tree, for prefix sums, range sums,
and point updates in `O(log n)` time.

## Layer 1

This package is part of Layer 1 of the coding-adventures computing stack.

## What It Includes

- `FenwickTree(length)` for zero-filled construction
- `FenwickTree.FromList` for linear-time building from existing values
- `Update`, `PrefixSum`, `RangeSum`, and `PointQuery`
- `FindKth` for prefix-sum order statistics
- Inline commentary on why `lowBit(index)` reveals each node's covered range

## Example

```fsharp
open CodingAdventures.FenwickTree

let tree = FenwickTree.FromList([ 3.0; 2.0; 1.0; 7.0; 4.0 ])

printfn "%f" (tree.PrefixSum(4)) // 13.000000
tree.Update(3, 5.0)
printfn "%f" (tree.PointQuery(3)) // 6.000000
```

## Development

```bash
# Run tests
bash BUILD
```
