# Changelog — asic-placement

## [0.1.0] — 2026-06-13

### Added
- `CellSize`, `PlacementOptions`, `PlacementReport`, `PlacementError` — placement data model.
- `place()` — row-packing initialization, HPWL-minimizing simulated annealing (swap-based, geometric cooling), optional legalization.
- `Xorshift64` — internal PRNG with `next()`, `usize_below()`, `f64_01()`; no external rand dependency.
- `find_row()`, `total_hpwl()`, `legalize()` helpers.
- 6 integration tests + 1 doc-test.
