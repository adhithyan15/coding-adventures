# Changelog

## [0.1.0] - 2026-03-20

### Added
- `CodingAdventures.Tree.Tree` struct backed by `DirectedGraph.Graph`
- Mutation: `add_child/3`, `remove_subtree/2`
- Queries: `root/1`, `parent/2`, `children/2`, `siblings/2`, `is_leaf?/2`, `is_root?/2`, `depth/2`, `height/1`, `size/1`, `nodes/1`, `leaves/1`, `has_node?/2`
- Tree traversals: `preorder/1`, `postorder/1`, `level_order/1`
- Utilities: `path_to/2` (root-to-node path), `lca/3` (lowest common ancestor), `subtree/2` (extract subtree)
- ASCII visualization via `to_ascii/1`
- `graph/1` function for accessing the underlying `DirectedGraph`
- Fully immutable -- all operations return new structs
