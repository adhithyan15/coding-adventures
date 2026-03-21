# Tree (Rust)

A rooted tree data structure backed by a directed graph, with traversals, lowest common ancestor, subtree extraction, and ASCII visualization.

## How It Fits in the Stack

This crate builds on top of the `directed-graph` crate. The directed graph handles all low-level node/edge storage, while this `Tree` type enforces tree invariants (single root, single parent per node, no cycles) and provides tree-specific operations.

## Usage

```rust
use tree::Tree;

let mut t = Tree::new("Program");
t.add_child("Program", "Assignment").unwrap();
t.add_child("Program", "Print").unwrap();
t.add_child("Assignment", "Name").unwrap();
t.add_child("Assignment", "BinaryOp").unwrap();

println!("{}", t.to_ascii());
// Program
// +-- Assignment
// |   +-- BinaryOp
// |   +-- Name
// +-- Print
```

## API

- `Tree::new(root)` -- create a tree with the given root
- `add_child(parent, child)` -- add a child under parent
- `remove_subtree(node)` -- remove a node and all descendants
- `root()`, `parent(node)`, `children(node)`, `siblings(node)`
- `is_leaf(node)`, `is_root(node)`, `depth(node)`, `height()`, `size()`
- `nodes()`, `leaves()`, `has_node(node)`
- `preorder()`, `postorder()`, `level_order()`
- `path_to(node)`, `lca(a, b)`, `subtree(node)`
- `to_ascii()` -- ASCII visualization
- `graph()` -- access the underlying Graph
