# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-09

### Added
- Initial Go implementation of the undirected graph package
- `Graph` struct with adjacency map representation (`map[string]map[string]bool`)
- Node operations: `AddNode`, `RemoveNode`, `HasNode`, `Nodes`, `Size`
- Edge operations: `AddEdge`, `RemoveEdge`, `HasEdge`, `Edges`
- Neighbor queries: `Neighbors`, `Degree`
- Error handling for all fallible operations
- Comprehensive unit test suite
- Knuth-style literate programming comments throughout
