# Changelog — coverage-hdl

## [0.1.0] — 2026-06-13

### Added

- `Bin` with `Arc<dyn Fn(i64) -> bool + Send + Sync>` matcher; constructors `bin_value`, `bin_range`, `bin_default`
- `Coverpoint` with first-match-wins sampling, per-bin hit counts, and `coverage() -> f64`
- `CrossPoint` for Cartesian-product coverage across multiple coverpoints; tracks last-sampled value per signal
- `ToggleStats` (`rising`, `falling`) and `CoverageReport`
- `CoverageRecorder` using `Arc<Mutex<RecorderInner>>` shared with a `vm.subscribe` callback
  - `add_coverpoint`, `add_cross`, `enable_toggle_coverage`, `sample_cross`, `report`, `overall_coverage`
- 19 integration tests + 5 doctests; all pass
- `hdl-ir` added as a dev-dependency (needed for integration test HIR builders)

### Notes

- Rust port of the Python `coverage_hdl` package
- `overall_coverage` averages coverpoint, cross, and toggle fractions equally
- Toggle coverage fires on 0→non-zero (rising) and non-zero→0 (falling) transitions
