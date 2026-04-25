# CodingAdventures.FenwickTree

An immutable Binary Indexed Tree for prefix sums and point updates.

```elixir
tree = CodingAdventures.FenwickTree.from_list([3, 2, 1, 7, 4])
CodingAdventures.FenwickTree.range_sum(tree, 2, 4)
```
