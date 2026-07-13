# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C++17, header-only port of the Rust `graph` crate in namespace
  `ca::graph`: an undirected weighted graph over string-named nodes, with
  heterogeneous property bags on the graph / nodes / edges and the standard
  algorithms.
- **Both representations** (`Repr::AdjacencyList` and `Repr::AdjacencyMatrix`)
  implemented faithfully, backed by `std::map` / `std::set` for the same ordered
  semantics as Rust's `BTreeMap` / `BTreeSet`.
- `Graph` methods (nodes/edges/neighbors, property getters & setters) plus the
  free-function algorithms `bfs`, `dfs`, `is_connected`, `connected_components`,
  `has_cycle`, `shortest_path` (BFS for unit weights, else Dijkstra), and
  `minimum_spanning_tree` (Kruskal + union-find).
- Idiomatic C++ surface: exceptions (`Error : std::runtime_error` carrying an
  `ErrorKind`) where the Rust crate returns `Result`, `std::optional`-based edge
  cells, and a faithful `total_cmp` (Rust `f64::total_cmp`) for deterministic
  edge ordering. Verified clean under ASan + UBSan.
- 64 checks (mirroring the crate's unit tests across both representations) run
  under every ISO C++ compiler via the shared `iso-harness`.
