# Tree

A rooted tree data structure backed by a [DirectedGraph](../directed-graph/). Trees are one of the most fundamental data structures in computer science — file systems, HTML documents, Abstract Syntax Trees (ASTs), and organization charts are all trees.

## What Is a Tree?

A tree is a directed graph with three constraints:

1. **Exactly one root** — a node with no parent
2. **Single parent** — every non-root node has exactly one parent
3. **No cycles** — you can never follow edges and return to where you started

Edges point from parent to child. A tree with N nodes always has exactly N-1 edges.

## Installation

```bash
pip install coding-adventures-tree
```

Or for development:

```bash
uv pip install -e ".[dev]"
```

## Quick Start

```python
from tree import Tree

# Build a tree representing an AST
t = Tree("Program")
t.add_child("Program", "Assignment")
t.add_child("Program", "Print")
t.add_child("Assignment", "Name")
t.add_child("Assignment", "BinaryOp")
t.add_child("BinaryOp", "Left")
t.add_child("BinaryOp", "Right")

# Visualize it
print(t.to_ascii())
# Program
# ├── Assignment
# │   ├── BinaryOp
# │   │   ├── Left
# │   │   └── Right
# │   └── Name
# └── Print
```

## API Overview

### Building Trees

```python
t = Tree("root")                    # Create with root node
t.add_child("root", "child")       # Add child under parent
t.remove_subtree("child")          # Remove node and all descendants
```

### Querying

```python
t.root                  # Root node name
t.parent("child")       # Parent of node (None for root)
t.children("node")      # List of children (sorted)
t.siblings("node")      # Other children of same parent
t.is_leaf("node")       # True if no children
t.is_root("node")       # True if root
t.depth("node")         # Distance from root (root = 0)
t.height()              # Maximum depth in tree
t.size()                # Total node count
t.nodes()               # All nodes (sorted)
t.leaves()              # All leaf nodes (sorted)
t.has_node("node")      # Existence check
len(t)                  # Same as size()
"node" in t             # Same as has_node()
```

### Traversals

```python
t.preorder()      # Parent before children (DFS)
t.postorder()     # Children before parent (DFS)
t.level_order()   # Breadth-first (BFS)
```

### Utilities

```python
t.path_to("node")       # Path from root to node: ["root", ..., "node"]
t.lca("a", "b")         # Lowest common ancestor
t.subtree("node")       # Extract subtree as new Tree
t.to_ascii()             # ASCII art visualization
t.graph                  # Access underlying DirectedGraph
```

## How It Fits in the Stack

The tree package sits above the directed-graph package in the dependency hierarchy:

```
tree  →  directed-graph
```

Trees are used by parsers (ASTs), file systems, and other hierarchical structures throughout the computing stack. The `Tree` class delegates all graph storage to `DirectedGraph` and adds tree-specific invariants and operations on top.

## Error Handling

```python
from tree import TreeError, NodeNotFoundError, DuplicateNodeError, RootRemovalError

# All tree errors inherit from TreeError
try:
    t.add_child("nonexistent", "child")
except NodeNotFoundError as e:
    print(e.node)  # "nonexistent"

try:
    t.add_child("root", "existing_child")
except DuplicateNodeError as e:
    print(e.node)  # "existing_child"

try:
    t.remove_subtree(t.root)
except RootRemovalError:
    print("Cannot remove root")
```

## Development

```bash
# Run tests
uv pip install -e ../directed-graph -e ".[dev]"
pytest tests/ -v

# Lint
ruff check src/ tests/
```
