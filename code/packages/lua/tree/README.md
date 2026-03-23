# tree

Rooted tree data structure backed by a directed graph, providing parent-child
relationships, traversals, lowest common ancestor, subtree extraction, and
ASCII visualization.

## What is a Tree?

A tree is a connected, acyclic graph where:

1. There is exactly one root node (no parent)
2. Every other node has exactly one parent
3. There are no cycles

Trees with N nodes always have exactly N-1 edges.

## How it Works

The tree is stored as a `DirectedGraph` from the `directed-graph` package, with
edges pointing from parent to child. This means `graph:successors(node)` returns
the children and `graph:predecessors(node)` returns 0 or 1 parent. The `Tree`
class enforces tree invariants on top of the graph.

## API

### Construction

```lua
local tree_mod = require("coding_adventures.tree")
local Tree = tree_mod.Tree

local t = Tree.new("Program")
t:add_child("Program", "Assignment")
t:add_child("Program", "Print")
t:add_child("Assignment", "Name")
t:add_child("Assignment", "BinaryOp")
```

### Queries

```lua
t:root()                -- "Program"
t:parent("Assignment")  -- "Program"
t:children("Program")   -- {"Assignment", "Print"}
t:siblings("Assignment") -- {"Print"}
t:is_leaf("Name")       -- true
t:is_root("Program")    -- true
t:depth("Name")         -- 2
t:height()              -- 2
t:size()                -- 5
t:nodes()               -- all nodes sorted
t:leaves()              -- leaf nodes sorted
t:has_node("Print")     -- true
```

### Traversals

```lua
t:preorder()     -- parent before children
t:postorder()    -- children before parent
t:level_order()  -- breadth-first, level by level
```

### Utilities

```lua
t:path_to("Name")               -- {"Program", "Assignment", "Name"}
t:lca("Name", "BinaryOp")       -- "Assignment"
local sub = t:subtree("Assignment")  -- new independent tree
```

### Visualization

```lua
print(t:to_ascii())
-- Program
-- +-- Assignment
-- |   +-- BinaryOp
-- |   +-- Name
-- +-- Print
```

### Mutation

```lua
t:add_child("Print", "StringLiteral")   -- add a node
t:remove_subtree("Assignment")          -- prune a branch
```

### Error Handling

Methods return `nil, error_table` on failure. Error tables have a `type` field:

- `"node_not_found"` -- referencing a node not in the tree
- `"duplicate_node"` -- adding a node that already exists
- `"root_removal"` -- attempting to remove the root

```lua
local ok, err = t:add_child("nonexistent", "child")
if not ok and err.type == "node_not_found" then
    print(err.message)
end
```

## Dependencies

- `coding-adventures-directed-graph` (>= 0.1.0)

## Development

```bash
# Run tests
cd tests
busted . --verbose --pattern=test_

# Or use the BUILD script
bash BUILD
```

## Position in the Stack

This package sits one layer above `directed-graph` in the computing stack.
While a directed graph is general-purpose, a tree adds the structural
constraints (single root, single parent, no cycles) that make it suitable
for representing hierarchical data like file systems, ASTs, and org charts.
