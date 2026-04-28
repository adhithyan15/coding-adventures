# Changelog

## Unreleased

### Added
- Graph, node, and edge property bags with canonical `weight` edge metadata.

## 0.1.0 — 2026-04-08

### Added
- `Graph[T]` class with adjacency-list and adjacency-matrix representations
- Node and edge operations: `add_node`, `remove_node`, `has_node`, `nodes`, `add_edge`, `remove_edge`, `has_edge`, `edges`, `edge_weight`
- Neighborhood queries: `neighbors`, `neighbors_weighted`, `degree`
- Pure-function algorithms: `bfs`, `dfs`, `is_connected`, `connected_components`, `has_cycle`, `shortest_path`, `minimum_spanning_tree`
- `GraphRepr` enum for selecting internal representation at construction time
- Comprehensive test suite with 95%+ coverage
