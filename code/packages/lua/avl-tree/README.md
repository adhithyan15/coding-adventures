# Lua AVL tree

A dependency-free persistent AVL tree with duplicate suppression, balanced
insertion and deletion, lookup, predecessor/successor queries, rank and k-th
order statistics, custom comparison, and metadata validation.

```lua
local AVLTree = require("coding_adventures.avl_tree").AVLTree

local tree = AVLTree.from_values({5, 1, 8, 3})
local updated = tree:delete(5)
assert(tree:contains(5))
assert(not updated:contains(5))
assert(updated:is_valid_avl())
```

`insert` and `delete` return new trees and preserve the original. Supply a
three-way comparison function to `empty` or `from_values` for values that do
not use Lua's default `<` and `>` ordering.

## Development

```bash
bash BUILD
```
