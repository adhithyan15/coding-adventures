# Changelog

All notable changes to `coding_adventures_directed_graph_native` will be
documented in this file.

## [0.1.0] - 2026-03-21

### Added

- Initial release: Rust-backed directed graph native extension for Ruby
- Built on `ruby-bridge` (zero-dependency Rust wrapper over Ruby's C API)
- All graph algorithms run in Rust: topological sort, cycle detection,
  transitive closure, affected nodes, independent groups
- Methods: `add_node`, `remove_node`, `has_node?`, `nodes`, `size`,
  `add_edge`, `remove_edge`, `has_edge?`, `edges`, `predecessors`,
  `successors`, `topological_sort`, `has_cycle?`, `transitive_closure`,
  `affected_nodes`, `independent_groups`
- extconf.rb generates a cargo-based Makefile (no rb-sys or mkmf dependency)
