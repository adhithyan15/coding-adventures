# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-21

### Added

- Initial release of the Rust-backed Ruby native extension for directed graphs.
- Magnus wrapper (`src/lib.rs`) exposing all Rust `Graph` methods to Ruby.
- Node operations: `add_node`, `remove_node`, `has_node?`, `nodes`.
- Edge operations: `add_edge`, `remove_edge`, `has_edge?`, `edges`.
- Neighbor queries: `predecessors`, `successors`.
- Graph properties: `size`, `inspect`, `to_s`.
- Algorithms: `topological_sort`, `has_cycle?`, `transitive_closure`, `affected_nodes`, `independent_groups`.
- Custom exception classes: `CycleError`, `NodeNotFoundError`, `EdgeNotFoundError`.
- Full test suite with 50+ tests mirroring the pure Ruby and Python native extension test suites.
- BUILD file for CI integration with the monorepo build system.
- README with usage examples and API reference table.
