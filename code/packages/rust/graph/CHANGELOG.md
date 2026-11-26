# graph changelog

## Unreleased

- Added `try_shortest_path` to reject stored negative and nonfinite weights before
  search; legacy `shortest_path` returns an empty path for invalid-weight graphs.
- Added deterministic property, replayable fuzz-engine, and bounded SAT checks for
  weighted shortest paths.

- Added graph, node, and edge property bags with canonical `weight` edge metadata.

## 0.1.0

- Added a weighted undirected graph with adjacency-list and adjacency-matrix storage
- Added BFS, DFS, connectivity, cycle detection, shortest path, and minimum spanning tree helpers
