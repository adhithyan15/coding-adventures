# Go Fenwick tree

A Binary Indexed Tree for O(log n) prefix sums and point updates over numeric
values.

```go
tree := fenwicktree.FromSlice([]float64{3, 2, 1, 7, 4})
sum, _ := tree.RangeSum(2, 4) // 10
tree.Update(3, 5)
```
