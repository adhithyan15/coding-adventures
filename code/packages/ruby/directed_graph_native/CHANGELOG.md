# Changelog

All notable changes to `coding_adventures_directed_graph_native` will be
documented in this file.

## [Unreleased]

### Fixed

- Package is now built and tested by the repo build tool. Added the build
  infrastructure that every other Ruby native extension already ships and
  that this package was missing:
  - `ext/directed_graph_native/build.rs` — emits the platform link flags a
    Ruby `cdylib` needs (`-undefined dynamic_lookup` on macOS; a synthesized
    MSVC import library on Windows). Without it the extension failed to link
    on macOS.
  - `BUILD` — `bundle install && rake compile && rake test`, so the build
    tool actually compiles and exercises the extension instead of skipping it.
  - `Gemfile` — required by the `bundle install` step in `BUILD`.
  - Brings the package in line with its sibling `graph_native` and the ten
    other Ruby native extensions.

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
