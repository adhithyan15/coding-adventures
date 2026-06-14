# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

### Added

- Inherited DT00 graph and node property bags on `DirectedGraph`.
- Added directed edge property bags keyed by ordered `(u, v)` edge identity.
- Synchronized the canonical `weight` edge property with directed edge weights
  and reverse-adjacency weights.

## [0.2.0] - 2026-04-08

### Changed (Breaking)

- **Rewritten to extend `Graph[T]` from DT00** (`coding-adventures-graph`).
  `DirectedGraph` now inherits from `Graph[T]` instead of being a standalone class.
- **Algorithms moved to module-level pure functions** in `algorithms.py`.
  Previously algorithms were methods on `DirectedGraph`; now they are standalone
  functions imported from `directed_graph.algorithms` (and re-exported from
  `directed_graph` top-level).
- **`CycleError` removed** — algorithms now raise `ValueError` on cycle detection
  (matches DT01 spec). A `CycleError = ValueError` alias is exported for backwards
  compatibility.
- **`NodeNotFoundError` and `EdgeNotFoundError` removed** — replaced by `KeyError`
  (base class behaviour from `Graph[T]`).
- **`topological_sort`, `has_cycle`, etc. are now functions, not methods**.
  Old: `g.topological_sort()`. New: `topological_sort(g)`.
- **`nodes()` returns `frozenset[T]`** (was `list`).
- **`successors()` and `predecessors()` return `frozenset[T]`** (was `list`).
- **`edges()` returns `frozenset[tuple[T, T, float]]`** including weights (was
  `list[tuple]` without weights).

### Added

- **`DirectedGraph(Graph[T])`** — inherits from DT00 `Graph[T]`; stores directed
  edges in inherited `_adj` (forward) plus new `_reverse` (reverse adjacency dict).
- **`out_degree(node)`** — number of outgoing edges from a node.
- **`in_degree(node)`** — number of incoming edges to a node.
- **`neighbors()` override** — returns successors only (forward edges), enabling
  `bfs`/`dfs` from `graph` package to traverse directed edges correctly.
- **`strongly_connected_components(graph)`** — Kosaraju's two-pass iterative DFS.
  Returns `list[frozenset[T]]`.
- **`LabeledDirectedGraph(Generic[T])`** — composition-based class with mandatory
  string labels on each edge. New method `edges_labeled()` returns
  `frozenset[tuple[T, T, str, float]]`.
- **Iterative DFS** — `has_cycle` and `strongly_connected_components` use explicit
  stack-based DFS to avoid Python's recursion limit on large graphs.
- **`allow_self_loops` parameter** — now a positional-or-keyword argument
  (was keyword-only `*`).
- **`coding-adventures-graph` dependency** — added to `pyproject.toml` and
  `BUILD`/`BUILD_windows` scripts.
- Comprehensive test suite in `tests/test_directed_graph.py` with 95%+ coverage.
  11 test classes covering all public APIs plus compatibility with Graph algorithms.

### Removed

- `graph.py` — replaced by `directed_graph.py` (new class structure).
- `labeled_graph.py` — replaced by `LabeledDirectedGraph` in `directed_graph.py`.
- `visualization.py` — removed (not part of DT01 spec; no `to_dot`/`to_mermaid`).
- `tests/test_graph.py`, `test_labeled_graph.py`, `test_algorithms.py`,
  `test_visualization.py` — replaced by `tests/test_directed_graph.py`.

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
