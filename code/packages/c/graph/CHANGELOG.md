# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `graph` crate: an undirected weighted graph over
  string-named nodes, with heterogeneous property bags on the graph / nodes /
  edges and the standard algorithms.
- Node/edge operations (`graph_add_node`/`_edge`, `_remove_`, `_has_`,
  `graph_edge_weight`, sorted `graph_nodes`/`graph_neighbors`/`graph_edges`),
  property getters & setters (`graph_get_*_property` / `graph_set_*_property`),
  and algorithms (`graph_bfs`, `graph_dfs`, `graph_is_connected`,
  `graph_connected_components`, `graph_has_cycle`, `graph_shortest_path`,
  `graph_minimum_spanning_tree`).
- Ordered-by-key internal maps (sorted dynamic arrays with binary search) that
  mirror Rust's `BTreeMap`, plus a faithful `total_cmp` for edge sorting, so all
  output matches the crate byte-for-byte.
- Both representation values (`GRAPH_ADJ_LIST` / `GRAPH_ADJ_MATRIX`) are exposed
  through `graph_repr`, backed by a single ordered-adjacency model (documented
  divergence: the two Rust layouts are observably identical).
- Status-returning API (`GraphStatus`) in place of the Rust `Result`/panics; all
  growable buffers guard `size_t` overflow. Verified clean under ASan + UBSan and
  the macOS `leaks` tool (0 leaks).
- 110 checks (mirroring the crate's unit tests across both representation values,
  plus a missing-start-node case) run under every ISO C compiler via the shared
  `iso-harness`.
