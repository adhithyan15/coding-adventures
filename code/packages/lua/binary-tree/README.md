# Lua binary tree

A dependency-free generic binary tree with node lookup, child lookup, four
traversals, shape predicates, height and size queries, sparse array conversion,
and ASCII rendering.

```lua
local binary_tree = require("coding_adventures.binary_tree")
local BinaryTree = binary_tree.BinaryTree
local NULL = binary_tree.NULL

local tree = BinaryTree.from_level_order({1, 2, 3, 4, NULL, 5, NULL})
assert(tree:inorder()[1] == 4)
assert(tree:height() == 2)
```

Lua tables cannot retain an interior `nil` array element. Use the exported
`NULL` sentinel for absent positions passed to `from_level_order`; `to_array`
uses the same sentinel so its result remains a dense, length-preserving array.
Node values themselves may be any non-`nil` value except `NULL`.

## Development

```bash
bash BUILD
```
