# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-09

### Added
- Initial Rust implementation of the undirected graph package
- `Graph` struct with adjacency map representation (`HashMap<String, HashSet<String>>`)
- Node operations: `add_node`, `remove_node`, `has_node`, `nodes`, `size`
- Edge operations: `add_edge`, `remove_edge`, `has_edge`, `edges`
- Neighbor queries: `neighbors`, `degree`
- `GraphError` enum with `NodeNotFound`, `EdgeNotFound`, `SelfLoop` variants
- Comprehensive unit tests
- Knuth-style literate programming comments throughout
