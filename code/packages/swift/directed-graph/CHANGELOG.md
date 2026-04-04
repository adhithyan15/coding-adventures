# Changelog

## 0.1.0 — 2026-04-04

### Added

- Initial release of the DirectedGraph Swift package.
- `Graph` struct with dual adjacency maps for O(1) neighbor lookups.
- Node operations: add, remove, has, list.
- Edge operations: add, remove, has, list.
- Neighbor queries: successors, predecessors.
- Algorithms: topological sort (Kahn's), cycle detection, transitive closure,
  transitive dependents, independent groups, affected nodes.
- Custom error types: `CycleError`, `NodeNotFoundError`, `EdgeNotFoundError`.
- Comprehensive test suite with 22 test cases.
