# Changelog

All notable changes to the directed-graph Go package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-18

### Added
- `Graph` type with forward + reverse adjacency maps
- Core operations: AddNode, AddEdge, RemoveNode, RemoveEdge, HasNode, HasEdge
- Query methods: Nodes, Edges, Predecessors, Successors, Size
- Topological sort via Kahn's algorithm
- Cycle detection via DFS 3-color marking
- Transitive closure (all downstream reachable nodes)
- Transitive dependents (all nodes that depend on a given node)
- Independent groups for parallel execution (modified Kahn's)
- Affected nodes computation for incremental builds
- Custom error types: CycleError, NodeNotFoundError, EdgeNotFoundError
- 39 tests including real repo dependency graph integration test
