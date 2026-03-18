# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-18

### Added

- `DirectedGraph` class with forward and reverse adjacency dict storage
- Core node operations: `add_node`, `remove_node`, `has_node`, `nodes`
- Core edge operations: `add_edge`, `remove_edge`, `has_edge`, `edges`
- Neighbor queries: `predecessors`, `successors`
- Dunder methods: `__len__`, `__contains__`, `__repr__`
- Topological sort using Kahn's algorithm with cycle detection
- Cycle detection using DFS three-color (white/gray/black) marking
- Transitive closure (forward BFS reachability)
- Transitive dependents (reverse BFS reachability)
- Independent groups (parallel execution levels via modified Kahn's)
- Affected nodes computation (changed set + transitive dependents)
- Custom exceptions: `CycleError` (with cycle path), `NodeNotFoundError`, `EdgeNotFoundError`
- Self-loop prevention (raises `ValueError`)
- Idempotent add operations (duplicate adds are no-ops)
- Full type annotations with py.typed marker
- Comprehensive test suite with 95%+ coverage targeting 14 test scenarios
- Integration test using real 21-package repository dependency graph
