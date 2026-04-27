# Changelog

## 1.0.0 — 2026-04-09

### Added

- Complete generic undirected weighted graph implementation with two representations:
  - Adjacency list (default, O(V+E) space)
  - Adjacency matrix (O(V²) space, O(1) edge lookup)
- Generic over any Hashable node type conforming to CustomStringConvertible
- Core operations: addNode, removeNode, hasNode, nodes, count, addEdge, removeEdge, hasEdge, edges, edgeWeight
- Neighbourhood queries: neighbors, neighborsWeighted, degree
- Graph traversal algorithms: bfs, dfs
- Connectivity: isConnected (computed property)
- Path algorithms: shortestPath (BFS or Dijkstra for weighted)
- Cycle detection: hasCycle()
- Minimum spanning tree: minimumSpanningTree() (Kruskal's algorithm with Union-Find)
- Bipartite checking: isBipartite() (BFS-based 2-coloring)
- Weighted edges with default weight 1.0
- Proper error handling with GraphError enum (throwing methods)
- Comprehensive test suite with 41 XCTest cases covering both representations
- Value semantics (Graph is a struct, not a class)
- Inline documentation with examples and theory
