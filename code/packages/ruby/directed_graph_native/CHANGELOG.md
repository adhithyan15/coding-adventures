# Changelog

All notable changes to `coding_adventures_directed_graph_native` will be
documented in this file.

## Unreleased - complete the Windows import-library symbol list

- `ext/**/build.rs` generates the MSVC import library for Ruby's DLL from a hand-written list of
  `rb_*` symbols. The list is now a superset of every symbol `ruby-bridge` declares (28 functions
  and 4 data symbols), so a symbol the bridge uses can never be missing at link time. Before, a
  crate whose list lagged the bridge failed to link on Windows with `rust-lld: undefined symbol:
  rb_str_new` / `rb_hash_new` / ... (`ruby/graph_native` in the scheduled full build; the same cause
  as the earlier `bitset_native`, `tree_set_native` and `board_vm_native` fixes). Every added name
  is exported by libruby. Validated by `cargo check` on Linux and by checking each name against
  libruby's exports; the Windows link itself is proven by the next daily Windows full build.

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
