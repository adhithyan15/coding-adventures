# coding_adventures_fenwick_tree

A Ruby Binary Indexed Tree for prefix sums and point updates.

```ruby
tree = CodingAdventures::FenwickTree::FenwickTree.from_list([3, 2, 1, 7, 4])
tree.range_sum(2, 4) #=> 10
tree.update(3, 5)
```
