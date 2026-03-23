# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- Tree class backed by DirectedGraph with metatable OOP pattern
- `Tree.new(root)` constructor — creates a single-node tree
- **Mutation:** `add_child(parent, child)`, `remove_subtree(node)`
- **Queries:** `root()`, `parent(node)`, `children(node)`, `siblings(node)`,
  `is_leaf(node)`, `is_root(node)`, `depth(node)`, `height()`, `size()`,
  `nodes()`, `leaves()`, `has_node(node)`
- **Traversals:** `preorder()`, `postorder()`, `level_order()`
- **Utilities:** `path_to(node)`, `lca(a, b)`, `subtree(node)`
- **Visualization:** `to_ascii()` — Unicode box-drawing tree rendering
- **Graph access:** `graph()` — returns the underlying DirectedGraph
- **String representation:** `__tostring` metamethod
- Structured error types: `NodeNotFoundError`, `DuplicateNodeError`, `RootRemovalError`
- Comprehensive busted test suite covering all methods and edge cases
- Ported from the Go `tree` package with idiomatic Lua conventions
