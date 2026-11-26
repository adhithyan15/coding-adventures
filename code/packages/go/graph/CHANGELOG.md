# Changelog

## Unreleased

- Made `ShortestPath` reject graphs containing negative, NaN, or infinite edge weights before query shortcuts or Dijkstra traversal. Stored weights and minimum-spanning-tree behavior remain unchanged.
- Added an exhaustive four-node simple-path model property, a negative control, and a time-bounded native fuzz target to `BUILD` and `BUILD_windows`.

- Added graph, node, and edge property bags with canonical `weight` edge metadata.

## 0.1.0

- Added the initial undirected weighted graph package with adjacency-list and adjacency-matrix representations.
- Added BFS, DFS, connectivity, cycle detection, shortest path, and minimum spanning tree algorithms.
