# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- **DirectedGraph** — core directed graph with forward/reverse adjacency maps
  - `new()` / `new_allow_self_loops()` constructors
  - Node operations: `add_node`, `remove_node`, `has_node`, `nodes`, `size`
  - Edge operations: `add_edge`, `remove_edge`, `has_edge`, `edges`
  - Neighbor queries: `predecessors`, `successors`
  - Topological sort via Kahn's algorithm (`topological_sort`)
  - Cycle detection via DFS three-color marking (`has_cycle`)
  - Transitive closure / dependents via BFS (`transitive_closure`, `transitive_dependents`)
  - Independent group partitioning for parallel execution (`independent_groups`)
  - Affected node analysis for change propagation (`affected_nodes`, `affected_nodes_list`)
  - Self-loop support (opt-in via `new_allow_self_loops`)
- **LabeledGraph** — directed graph with labeled edges (composition over DirectedGraph)
  - All DirectedGraph operations with label-aware edge semantics
  - `add_edge(from, to, label)` with multi-label support per edge
  - `remove_edge(from, to, label)` removes specific label; removes edge when last label gone
  - `has_edge_with_label`, `labels` queries
  - `successors_with_label`, `predecessors_with_label` filtered queries
  - Algorithm delegation to underlying DirectedGraph
  - `graph()` accessor for direct access to underlying DirectedGraph
- **Visualization** module with three output formats:
  - `to_dot` / `labeled_to_dot` — Graphviz DOT format with DotOptions
  - `to_mermaid` / `labeled_to_mermaid` — Mermaid flowchart format
  - `to_ascii_table` / `labeled_to_ascii_table` — plain-text adjacency/transition tables
- **Error types** — structured error tables with `type` discriminator:
  - `CycleError`, `NodeNotFoundError`, `EdgeNotFoundError`, `LabelNotFoundError`
- Comprehensive busted test suite with 100+ test cases
- Literate programming style with inline documentation
