# Changelog

## [0.1.0] — Unreleased

### Added
- `place(fp, cell_sizes, nets=None, options=None) -> (Def, PlacementReport)`:
  - Random row + sequential within-row initial placement.
  - Simulated annealing on total HPWL with exponential cooling (T0 = HPWL / N).
  - Greedy row legalization: sort by x, pack left-to-right.
- `CellSize(cell_type, width, height)`.
- `PlacementOptions(method, iterations, seed, target_density, legalize)`.
- `PlacementReport(final_hpwl, cells_placed, runtime_sec, accepted_swaps, rejected_swaps)`.
- HPWL cost: sum over nets of (max_x - min_x) + (max_y - min_y).

### Out of scope (v0.2.0)
- Analytical placement (quadratic / RC-tree solver).
- Detailed placement (Abacus / tetris / RowLeg).
- Timing-driven placement.
- Region / pre-placed constraints.
