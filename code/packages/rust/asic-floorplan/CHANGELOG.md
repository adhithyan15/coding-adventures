# Changelog — asic-floorplan

## [0.1.0] — 2026-06-13

### Added
- `CellInstanceEstimate`, `IoSpec`, `FloorplanOptions`, `Floorplan`, `FloorplanError` — floorplan data model.
- `FloorplanOptions::sky130_hd()` — preconfigured defaults for Sky130 HD (site 0.46×2.72 µm, utilization 0.7, aspect 1.0, IO ring 10 µm).
- `compute_floorplan()` — core area from utilization, die sizing with IO ring, row generation, IO pin placement.
- `place_io_pins()` — inputs left, outputs right, others bottom.
- `floorplan_to_def()` — converts `Floorplan` → `Def` (uses `lef-def` types).
- 10 integration tests + 1 doc-test.
