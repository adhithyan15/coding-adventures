### Added — SPICE Berkeley Mosaic Host Wire Export
- `spice-netlist-parser` now exposes schema-versioned Berkeley app host-surface
  wire snapshots for Mosaic packaging and WebAssembly embedding.
  `host_surface_wire()`, `run_host_surface_wire()`, and their JSON helpers
  flatten the host panel contract into stable lower-case panel kinds,
  diagnostics, active-panel IDs, and repaired persisted editor-state metadata.
- The JSON helpers avoid exposing simulator internals to product shells while
  preserving the Rust app substrate over the public Berkeley parser contract.

