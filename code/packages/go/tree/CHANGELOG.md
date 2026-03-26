# Changelog

## 0.1.0 (2026-03-20)

### Added
- Initial release of the tree package for Go
- `Tree` type backed by `directedgraph.Graph`
- Tree construction with `AddChild` and pruning with `RemoveSubtree`
- Query methods: `Parent`, `Children`, `Siblings`, `IsLeaf`, `IsRoot`, `Depth`, `Height`, `Size`, `Nodes`, `Leaves`, `HasNode`
- Traversals: `Preorder`, `Postorder`, `LevelOrder`
- Utilities: `PathTo`, `LCA` (lowest common ancestor), `Subtree` extraction
- ASCII visualization with `ToAscii`
- Custom error types: `NodeNotFoundError`, `DuplicateNodeError`, `RootRemovalError`
- 60+ tests covering all methods and edge cases
