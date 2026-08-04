# Lua binary search tree

A dependency-free persistent binary search tree with duplicate suppression,
deletion, lookup, predecessor/successor queries, rank and k-th order statistics,
balanced construction from sorted arrays, and metadata validation.

```lua
local BinarySearchTree = require("coding_adventures.binary_search_tree").BinarySearchTree

local tree = BinarySearchTree.empty():insert(5):insert(1):insert(8):insert(3)
local updated = tree:delete(5)
assert(tree:contains(5))
assert(not updated:contains(5))
assert(updated:kth_smallest(2) == 3)
```

`insert` and `delete` return new trees and preserve the original. Supply a
three-way comparison function to `empty` or `from_sorted_array` for values that
do not use Lua's default `<` and `>` ordering.

## Development

```bash
bash BUILD
```
