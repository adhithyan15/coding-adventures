# Tree (Ruby)

A rooted tree data structure backed by a directed graph, with traversals, lowest common ancestor, subtree extraction, and ASCII visualization.

## How It Fits in the Stack

This package builds on top of `coding_adventures_directed_graph`. The directed graph handles all low-level node/edge storage, while this `Tree` class enforces tree invariants (single root, single parent per node, no cycles) and provides tree-specific operations.

## Installation

```ruby
gem "coding_adventures_tree", path: "code/packages/ruby/tree"
```

## Usage

```ruby
require "coding_adventures_tree"

t = CodingAdventures::Tree::Tree.new("Program")
t.add_child("Program", "Assignment")
t.add_child("Program", "Print")
t.add_child("Assignment", "Name")
t.add_child("Assignment", "BinaryOp")

puts t.to_ascii
# Program
# +-- Assignment
# |   +-- BinaryOp
# |   +-- Name
# +-- Print

t.preorder   # => ["Program", "Assignment", "BinaryOp", "Name", "Print"]
t.postorder  # => ["BinaryOp", "Name", "Assignment", "Print", "Program"]
t.depth("Name")  # => 2
t.lca("Name", "Print")  # => "Program"
```

## API

- `Tree.new(root)` -- create a tree with the given root
- `add_child(parent, child)` -- add a child under parent
- `remove_subtree(node)` -- remove a node and all descendants
- `root`, `parent(node)`, `children(node)`, `siblings(node)`
- `leaf?(node)`, `root?(node)`, `depth(node)`, `height`, `size`
- `nodes`, `leaves`, `has_node?(node)`
- `preorder`, `postorder`, `level_order`
- `path_to(node)`, `lca(a, b)`, `subtree(node)`
- `to_ascii` -- ASCII visualization
- `graph` -- access the underlying DirectedGraph
