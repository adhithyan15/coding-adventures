# Changelog

All notable changes to the directed-graph-native package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - Unreleased

### Added
- PyO3 wrapper around the Rust `directed-graph` crate
- `DirectedGraph` class with full API matching the pure Python version
- Custom exception types: `CycleError`, `NodeNotFoundError`, `EdgeNotFoundError`
- All algorithms: `topological_sort`, `has_cycle`, `transitive_closure`, `affected_nodes`, `independent_groups`
- Python dunder methods: `__len__`, `__contains__`, `__repr__`
- maturin-based build system for cross-platform wheel generation
- Comprehensive test suite mirroring the pure Python tests
