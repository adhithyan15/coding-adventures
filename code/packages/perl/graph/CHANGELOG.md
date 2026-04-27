# Changelog

All notable changes to this package will be documented in this file.

## [1.0.0] - 2026-04-09

### Added

- Complete undirected weighted graph implementation with two representations:
  - Adjacency list (default, O(V+E) space)
  - Adjacency matrix (O(V²) space, O(1) edge lookup)
- Core operations: add_node, remove_node, has_node, nodes, add_edge, remove_edge, has_edge, edges, edge_weight
- Neighbourhood queries: neighbors, neighbors_weighted, degree
- Graph traversal algorithms: bfs, dfs
- Connectivity algorithms: is_connected, connected_components
- Path algorithms: shortest_path (BFS or Dijkstra for weighted)
- Cycle detection: has_cycle
- Minimum spanning tree: minimum_spanning_tree (Kruskal's algorithm)
- Bipartite checking: is_bipartite (BFS-based 2-coloring)
- Weighted edges with default weight 1.0
- Proper error handling with Carp::croak
- Comprehensive test suite with 18 test groups covering both representations
- POD documentation for all public methods
