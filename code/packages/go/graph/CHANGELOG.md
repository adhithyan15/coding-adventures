# Changelog

## Unreleased

- Added `ConcurrentGraph`, an opt-in synchronized owner with detached read-only snapshots, independent edge cursors, and consistent behavior when handles are copied.
- Added bounded sequential-model histories, copied-handle reader/writer schedules, snapshot ownership checks, and a time-bounded concurrent graph fuzz target.
- Fixed edge metadata identity and adjacency-list edge deduplication for distinct Go string endpoint pairs containing embedded NUL bytes; added pair-model and snapshot regressions.

- Made `ShortestPath` reject graphs containing negative, NaN, or infinite edge weights before query shortcuts or Dijkstra traversal. Stored weights and minimum-spanning-tree behavior remain unchanged.
- Added an exhaustive four-node simple-path model property, a negative control, and a time-bounded native fuzz target to `BUILD` and `BUILD_windows`.
- Distinguished topologically reachable paths with no finite computed `float64` distance from truly unreachable targets; added a bounded overflow model property, fuzz target, and representation regressions.
- Added opt-in `AddEdgeChecked` and `SetEdgePropertyChecked` methods for finite non-negative weight validation while preserving legacy mutation behavior.

- Added graph, node, and edge property bags with canonical `weight` edge metadata.

## 0.1.0

- Added the initial undirected weighted graph package with adjacency-list and adjacency-matrix representations.
- Added BFS, DFS, connectivity, cycle detection, shortest path, and minimum spanning tree algorithms.
