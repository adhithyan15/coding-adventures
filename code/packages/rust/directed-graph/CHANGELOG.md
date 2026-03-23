# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- Initial Rust port of the directed-graph package
- `Graph` struct with forward and reverse adjacency maps (`HashMap<String, HashSet<String>>`)
- Node operations: `add_node`, `remove_node`, `has_node`, `nodes`, `size`
- Edge operations: `add_edge`, `remove_edge`, `has_edge`, `edges`
- Neighbor queries: `predecessors`, `successors`
- Topological sort using Kahn's algorithm
- Cycle detection using DFS three-color algorithm
- Transitive closure via BFS
- Affected nodes computation for build system change detection
- Independent groups for parallel execution levels
- `GraphError` enum with `CycleError`, `NodeNotFound`, `EdgeNotFound`, `SelfLoop` variants
- Manual `Display` and `std::error::Error` implementations (zero external dependencies)
- Comprehensive test suite ported from Go implementation
- Knuth-style literate programming comments throughout
