# Changelog — standard-cell-library

## [0.1.0] — 2026-06-13

### Added
- `LookupTable` — 2-D NLDM grid with bilinear interpolation; out-of-range clamped.
- `TimingArc` — one arc per (input, output, sense) triple, with `cell_rise/fall` and `rise/fall_transition` LUTs.
- `CellTiming` — area, leakage, pin capacitances, and timing arcs per cell.
- `Library` with `get()`, `list_drives()` methods.
- `build_default_library()` — hand-curated NLDM tables for 35 Sky130 HD teaching cells; tuned to within ~10% of reference characterization.
- `select_drive()` — picks smallest drive strength meeting a delay budget at a given load; falls back to largest on impossible budget.
- 16 integration tests + 1 doc-test.
