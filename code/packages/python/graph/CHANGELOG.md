# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-04-09

### Added

- Complete Graph class with two representations: adjacency list (default) and adjacency matrix
- Support for weighted and unweighted undirected graphs
- Custom exception classes: NodeNotFoundError, EdgeNotFoundError
- Node operations: add_node, remove_node, has_node, nodes
- Edge operations: add_edge, remove_edge, has_edge, edges, edge_weight
- Neighborhood queries: neighbors, neighbors_weighted, degree
- Breadth-First Search (BFS) algorithm for level-order traversal
- Depth-First Search (DFS) algorithm for deep-first traversal
- Graph connectivity: is_connected, connected_components
- Cycle detection: has_cycle
- Shortest path algorithms: shortest_path (BFS for unweighted, Dijkstra for weighted)
- Minimum Spanning Tree (MST) using Kruskal's algorithm with Union-Find
- Union-Find (Disjoint Set Union) data structure for MST and cycle detection
- 97.38% test coverage (133 tests) exceeding 95% requirement
- Comprehensive docstrings with algorithm explanations and time complexity analysis
- Literate programming style with diagrams and analogies for clarity
