### Added — SPICE Berkeley Mosaic Host Surface
- `spice-netlist-parser` now exposes Berkeley app-deck host surfaces for Mosaic
  shell integration. `host_surface()` and `run_host_surface()` derive stable
  source, diagnostics, analysis, table, and waveform panel descriptors from
  persisted editor state, including panel IDs, target names, enabled states,
  active state, and disabled reasons.
- The surface stays Rust-only app substrate over the public Berkeley parser
  contract, so Python and TypeScript remain aligned when parser behavior
  changes while Mosaic hosts can wire panels without reinterpreting simulator
  internals.

