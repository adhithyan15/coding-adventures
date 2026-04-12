# lua/b-plus-tree — B+ Tree (DT12)

A B+ Tree implementation in pure Lua that maps keys to arbitrary values.

## What is a B+ Tree?

A B+ Tree is a variant of the B-Tree that keeps all data in leaf nodes.
Internal nodes hold only separator keys used for routing.
Leaf nodes are connected by a singly-linked list, enabling efficient range scans.

```
Internal:        [3]
                /    \
Leaves:  [1,2] ──▶ [3,4,5]
          ↑↑         ↑↑↑
        values       values
```

Key 3 appears in both the internal node AND the left-most position of the right leaf.
This is the crucial difference from a B-Tree, where the median key moves up and
disappears from the children.

## API

```lua
local BPlusTree = require("coding_adventures.b_plus_tree")

local tree = BPlusTree.new({ t = 2 })

tree:insert(1, "one")
tree:insert(2, "two")
tree:insert(3, "three")

tree:search(2)                         -- "two"
tree:search(99)                        -- nil

tree:min_key()                         -- 1
tree:max_key()                         -- 3

local all = tree:full_scan()           -- { {1,"one"}, {2,"two"}, {3,"three"} }
local r   = tree:range_scan(1, 2)      -- { {1,"one"}, {2,"two"} }

tree:delete(2)                         -- true  (found)
tree:delete(99)                        -- false (not found)

tree:size()                            -- 2
tree:height()                          -- 0
tree:is_valid()                        -- true
```

## Stack position

Standalone data structure (DT12). The plain B-Tree lives at `lua/b-tree` (DT11).
