# Changelog — tech-mapping

## [0.1.0] — 2026-06-13

### Added
- `CellMapEntry` — maps a generic cell name to a Sky130 stdcell name plus pin remap table.
- `default_sky130_map()` — 27-entry table covering all cells from `gate-netlist-format` BUILTIN_CELL_TYPES.
- `TechMapper` struct with `map()` and `map_module()` — renames cells and remaps pins.
- `cancel_inv_pairs()` — eliminates back-to-back `sky130_fd_sc_hd__inv_1` pairs.
- `MappingReport` — reports cells_before, cells_after, bubbles_canceled, unmapped list.
- `map_to_sky130()` — convenience top-level function.
- 9 unit tests covering all major behaviors.
