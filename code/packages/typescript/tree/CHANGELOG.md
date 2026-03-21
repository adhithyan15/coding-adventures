# Changelog

## 0.1.0 (2026-03-20)

### Added
- Initial release of the tree package for TypeScript
- `Tree` class backed by `Graph` from `@coding-adventures/directed-graph`
- Tree construction with `addChild` and pruning with `removeSubtree`
- Query methods: `parent`, `children`, `siblings`, `isLeaf`, `isRoot`, `depth`, `height`, `size`, `nodes`, `leaves`, `hasNode`
- Traversals: `preorder`, `postorder`, `levelOrder`
- Utilities: `pathTo`, `lca` (lowest common ancestor), `subtree` extraction
- ASCII visualization with `toAscii`
- Custom error types: `TreeError`, `NodeNotFoundError`, `DuplicateNodeError`, `RootRemovalError`
- 80+ tests covering all methods and edge cases
