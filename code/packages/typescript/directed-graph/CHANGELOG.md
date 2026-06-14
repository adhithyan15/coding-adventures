# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

### Added

- Added graph, node, and directed edge property bags using JSON-like scalar values.
- Added weighted directed edge storage with `edgeWeight`, `edgesWeighted`, and `successorsWeighted`.
- Synchronized the canonical `weight` edge property with directed edge weights.

### Changed

- Re-adding an existing directed edge now updates its weight and merges edge metadata.
- TypeScript build now limits compilation to `src/**/*.ts` and emits to `dist`.

## [0.1.0] - 2026-03-19

### Added

- `Graph` class with forward and reverse adjacency map storage (`Map<string, Set<string>>`)
- Core node operations: `addNode`, `removeNode`, `hasNode`, `nodes`
- Core edge operations: `addEdge`, `removeEdge`, `hasEdge`, `edges`
- Neighbor queries: `predecessors`, `successors`
- Utility methods: `size` getter, `toString`
- Topological sort using Kahn's algorithm with cycle detection
- Cycle detection using DFS three-color (white/gray/black) marking
- Transitive closure (forward BFS reachability)
- Transitive dependents (reverse BFS reachability)
- Independent groups (parallel execution levels via modified Kahn's)
- Affected nodes computation (changed set + transitive dependents)
- Custom error classes: `CycleError` (with cycle path), `NodeNotFoundError`, `EdgeNotFoundError`
- Self-loop prevention (throws `Error`)
- Idempotent add operations (duplicate adds are no-ops)
- Comprehensive test suite with vitest
- Integration test using real 21-package repository dependency graph
- TypeScript port of the Python `coding-adventures-directed-graph` package
