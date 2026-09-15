### Added — SPICE Berkeley Mosaic App Package Manifest
- `spice-netlist-parser` now exposes a schema-versioned Berkeley Mosaic app
  package manifest plus JSON helper for WebAssembly and product-shell
  packaging. The manifest advertises the Berkeley grammar version,
  host-surface wire schema, source-fingerprint algorithm, panel kinds, editor
  action kinds, command targets, runnable analysis directives, and artifact
  capabilities before a host opens a deck.
- The manifest keeps packaging metadata derived from the same Rust app facade
  contract as host surfaces and host-wire exports, avoiding a separate product
  registry while the public parser contract remains language-aligned.

