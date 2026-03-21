# Changelog

## [0.1.0] - 2026-03-20

### Added
- `Tree` class backed by `DirectedGraph`
- Mutation: `add_child`, `remove_subtree`
- Queries: `root`, `parent`, `children`, `siblings`, `is_leaf`, `is_root`, `depth`, `height`, `size`, `nodes`, `leaves`, `has_node`
- Tree traversals: `preorder`, `postorder`, `level_order`
- Utilities: `path_to` (root-to-node path), `lca` (lowest common ancestor), `subtree` (extract subtree)
- ASCII visualization via `to_ascii`
- Custom exceptions: `TreeError`, `NodeNotFoundError`, `DuplicateNodeError`, `RootRemovalError`
- `graph` property for accessing the underlying `DirectedGraph`
- `__len__` and `__contains__` dunder methods
