# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-09

### Added
- Initial Elixir implementation of the undirected graph package
- `Graph` struct with adjacency map representation (`map[node -> MapSet[neighbors]]`)
- Node operations: `add_node`, `remove_node`, `has_node?`, `nodes`, `size`
- Edge operations: `add_edge`, `remove_edge`, `has_edge?`, `edges`
- Neighbor queries: `neighbors`, `degree`
- Immutable API — all operations return new graph or error tuple
- Error handling with `{:ok, value}` and `{:error, reason}` tuples
- Comprehensive test suite
- Knuth-style literate programming comments throughout
