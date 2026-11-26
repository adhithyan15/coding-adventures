# graph-wasm changelog

## Unreleased

- Added a regression check that invalid shortest-path weights preserve the
  wrapper's empty-array result shape.
- Added JSON-based graph, node, and edge property bag bindings.

## 0.1.0

- Added WebAssembly bindings for the Rust `graph` crate
- Added browser-facing graph methods plus traversal, connectivity, shortest path, and MST helpers
