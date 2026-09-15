### Added — SPICE Berkeley Mosaic App Startup Summary
- `spice-netlist-parser` now exposes Berkeley Mosaic app startup summaries plus
  JSON helpers. The summary derives a compact ready/blocked route from the
  bootstrap payload, including package name, source fingerprint, repaired
  editor-state IDs, stale-state flags, active panel, diagnostic count, and
  blocking reason.
- The summary helpers reuse the run and non-run bootstrap paths so product
  shells can make startup routing decisions without walking the full host
  panel payload or duplicating simulator internals.

