# lua/b-tree — B-Tree (DT11)

A B-Tree implementation in pure Lua that maps keys to arbitrary values.

## What is a B-Tree?

A B-Tree is a self-balancing search tree invented at Boeing Research Labs in 1970.
It is the data structure powering virtually every database and filesystem.

## API

```lua
local BTree = require("coding_adventures.b_tree")

local tree = BTree.new({ t = 2 })

tree:insert(10, "apple")
tree:insert(20, "banana")

tree:search(10)          -- "apple"
tree:search(99)          -- nil

tree:min_key()           -- 10
tree:max_key()           -- 20

local all = tree:inorder()      -- { {10, "apple"}, {20, "banana"} }
local r   = tree:range_query(5, 15)   -- { {10, "apple"} }

tree:delete(10)          -- true  (found)
tree:delete(99)          -- false (not found)

tree:size()              -- 1
tree:height()            -- 0
tree:is_valid()          -- true
```

## Stack position

Standalone data structure (DT11). The B+ Tree lives at `lua/b-plus-tree` (DT12).
