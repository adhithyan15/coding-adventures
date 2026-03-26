# Changelog

All notable changes to the directed-graph Go package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-03-26

### Added
- `gen_capabilities.go`: self-contained Operations infrastructure generated per-package
  - `OperationResult[T]`: three-state outcome (success, expected failure, unexpected failure)
  - `ResultFactory[T]`: `Generate()` for common results; `Fail()` for typed expected errors
  - `Operation[T]`: unit of work with timing, structured logging, and panic recovery
  - `PanicOnUnexpected()`: opt-in re-panic for operations that signal errors via panics
  - `StartNew[T]()`: constructs an Operation without executing it
  - `_capabilityViolationError`: returned when an undeclared OS operation is attempted
- All public graph methods now wrapped in `StartNew` internally:
  - Every call is timed, logged as structured JSON, and panic-safe
  - Typed error returns preserved via `rf.Fail()` — `errors.As` checks unaffected
  - `AddEdge` uses `PanicOnUnexpected()` so self-loop panics still propagate to callers
  - Public API and all existing tests unchanged

### Changed
- Internal function bodies replaced with Operation callbacks; no change to callers
- `capability-cage-generator` updated to emit self-contained Operations code
  with no dependency on any shared package (supply chain isolation)
- OS capabilities exposed as namespace fields on `Operation[T]` itself (`op.File`,
  `op.Net`, etc.) rather than a separate injected `Cage` parameter — fields only
  exist when declared in `required_capabilities.json`, enforced at compile time
- Wildcards rejected for scopeable categories (fs, net, proc, env, ffi) at
  generation time — exact paths required

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
