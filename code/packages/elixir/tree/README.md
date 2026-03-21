# Tree (Elixir)

A rooted tree data structure backed by a [DirectedGraph](../directed_graph/). Trees are one of the most fundamental data structures in computer science -- file systems, HTML documents, Abstract Syntax Trees (ASTs), and organization charts are all trees.

## What Is a Tree?

A tree is a directed graph with three constraints:

1. **Exactly one root** -- a node with no parent
2. **Single parent** -- every non-root node has exactly one parent
3. **No cycles** -- you can never follow edges and return to where you started

Edges point from parent to child. A tree with N nodes always has exactly N-1 edges.

## Installation

Add the dependency in your `mix.exs`:

```elixir
defp deps do
  [{:coding_adventures_tree, path: "../tree"}]
end
```

## Quick Start

```elixir
alias CodingAdventures.Tree.Tree

tree = Tree.new("Program")
{:ok, tree} = Tree.add_child(tree, "Program", "Assignment")
{:ok, tree} = Tree.add_child(tree, "Program", "Print")
{:ok, tree} = Tree.add_child(tree, "Assignment", "Name")
{:ok, tree} = Tree.add_child(tree, "Assignment", "BinaryOp")

IO.puts(Tree.to_ascii(tree))
# Program
# ├── Assignment
# │   ├── BinaryOp
# │   └── Name
# └── Print
```

## API Overview

### Building Trees

```elixir
tree = Tree.new("root")
{:ok, tree} = Tree.add_child(tree, "root", "child")
{:ok, tree} = Tree.remove_subtree(tree, "child")
```

### Querying

```elixir
Tree.root(tree)              # Root node name
Tree.parent(tree, "child")   # {:ok, parent} or {:ok, nil}
Tree.children(tree, "node")  # {:ok, [sorted children]}
Tree.siblings(tree, "node")  # {:ok, [siblings]}
Tree.is_leaf?(tree, "node")  # true/false
Tree.is_root?(tree, "node")  # true/false
Tree.depth(tree, "node")     # {:ok, depth}
Tree.height(tree)             # max depth
Tree.size(tree)               # node count
Tree.nodes(tree)              # sorted list
Tree.leaves(tree)             # sorted leaf list
Tree.has_node?(tree, "node") # true/false
```

### Traversals

```elixir
Tree.preorder(tree)     # Parent before children
Tree.postorder(tree)    # Children before parent
Tree.level_order(tree)  # Breadth-first
```

### Utilities

```elixir
Tree.path_to(tree, "node")    # {:ok, [root, ..., node]}
Tree.lca(tree, "a", "b")      # {:ok, ancestor}
Tree.subtree(tree, "node")    # {:ok, new_tree}
Tree.to_ascii(tree)            # ASCII art string
Tree.graph(tree)               # underlying DirectedGraph
```

## Immutability

All operations return new Tree structs. The original tree is never modified. This is idiomatic Elixir and works well with concurrent code.

## How It Fits in the Stack

```
tree  ->  directed_graph
```

Trees are used by parsers (ASTs), file systems, and other hierarchical structures. The `Tree` module delegates graph storage to `DirectedGraph.Graph` and adds tree-specific invariants and operations on top.

## Development

```bash
mix deps.get
mix test --cover
```
