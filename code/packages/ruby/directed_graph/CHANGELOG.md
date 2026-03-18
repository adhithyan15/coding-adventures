# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-18

### Added

- `Graph` class with forward and reverse adjacency hash storage
- Core operations: `add_node`, `add_edge`, `remove_node`, `remove_edge`
- Query methods: `has_node?`, `has_edge?`, `nodes`, `edges`, `predecessors`, `successors`, `size`
- Kahn's algorithm topological sort (`topological_sort`)
- Cycle detection (`has_cycle?`)
- Transitive closure computation (`transitive_closure`)
- Transitive dependents query (`transitive_dependents`)
- Parallel execution levels (`independent_groups`)
- Incremental rebuild helper (`affected_nodes`)
- Custom errors: `CycleError`, `NodeNotFoundError`, `EdgeNotFoundError`
- Self-loop prevention (raises `CycleError`)
- Minitest test suite with 95%+ coverage
