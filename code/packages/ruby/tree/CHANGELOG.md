# Changelog

## 0.1.0 (2026-03-20)

### Added
- Initial release of the tree package for Ruby
- `Tree` class backed by `CodingAdventures::DirectedGraph::Graph`
- Tree construction with `add_child` and pruning with `remove_subtree`
- Query methods: `parent`, `children`, `siblings`, `leaf?`, `root?`, `depth`, `height`, `size`, `nodes`, `leaves`, `has_node?`
- Traversals: `preorder`, `postorder`, `level_order`
- Utilities: `path_to`, `lca` (lowest common ancestor), `subtree` extraction
- ASCII visualization with `to_ascii`
- Custom error types: `TreeError`, `NodeNotFoundError`, `DuplicateNodeError`, `RootRemovalError`
- 80+ tests covering all methods and edge cases
