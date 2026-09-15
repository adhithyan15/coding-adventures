### Added — SPICE Berkeley Mosaic App Bootstrap Snapshot
- `spice-netlist-parser` now exposes schema-versioned Berkeley Mosaic app
  bootstrap snapshots plus JSON helpers. The bootstrap payload combines the
  static package manifest with the deck-specific host-surface wire export so
  WebAssembly and product shells can load package capabilities, repaired
  editor-state metadata, active panels, diagnostics, and run availability from
  one startup envelope.
- The run and non-run helpers preserve the same blocked-deck diagnostic surface
  as host-wire exports while keeping the package manifest stable and derived
  from the Rust app facade contract.

