# Changelog

## 0.1.0 (2026-03-20)

### Added
- Initial release of the tree package for Rust
- `Tree` type backed by `directed_graph::Graph`
- Tree construction with `add_child` and pruning with `remove_subtree`
- Query methods: `parent`, `children`, `siblings`, `is_leaf`, `is_root`, `depth`, `height`, `size`, `nodes`, `leaves`, `has_node`
- Traversals: `preorder`, `postorder`, `level_order`
- Utilities: `path_to`, `lca` (lowest common ancestor), `subtree` extraction
- ASCII visualization with `to_ascii`
- Custom error type `TreeError` with variants: `NodeNotFound`, `DuplicateNode`, `RootRemoval`
- 60+ tests covering all methods and edge cases
