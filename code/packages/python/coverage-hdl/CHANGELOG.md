# Changelog

## [0.1.0] — Unreleased

### Added
- `Coverpoint(name, signal, bins=...)`: watches one signal, hits accumulate in matching bins.
- `Bin` + helpers: `bin_value(name, v)`, `bin_range(name, lo, hi)`, `bin_default()`.
- `CrossPoint(name, coverpoints)`: cross-product of bins across multiple coverpoints; sampled via `recorder.sample_cross(name=None)`.
- `CoverageRecorder(vm)`: subscribes to HardwareVM events. Auto-samples coverpoints on relevant signal changes; tracks toggle counts via `enable_toggle_coverage([signals])`.
- `CoverageReport`: dict-of-dict hits + per-signal `ToggleStats`.
- `overall_coverage` property: average across all coverpoints + crosses (value in [0, 1]).

### Out of scope (v0.2.0)
- Code coverage (line/branch/path) — needs HIR provenance instrumentation in the simulation path.
- FSM-state and FSM-transition coverage helpers.
- HTML reports.
- MC/DC analysis.
- Coverage merging.
