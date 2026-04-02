# Changelog

## 0.2.0 (2026-04-02)

### Changed

- **Operations pattern**: Wrapped all public methods with `StartNew` for automatic timing, structured logging, and panic recovery. Methods covered: `New`, `AddChild`, `RemoveSubtree`, `Root`, `HasNode`, `Height`, `Size`, `Parent`, `Children`, `Siblings`, `IsLeaf`, `IsRoot`, `Depth`, `Nodes`, `Leaves`, `Preorder`, `Postorder`, `LevelOrder`, `PathTo`, `LCA`, `Subtree`, `ToAscii`, `String`, `Graph`. The public API is fully backward-compatible.

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
