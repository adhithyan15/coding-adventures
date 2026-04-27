# Changelog

## [0.1.0] — Unreleased

Initial implementation of the HNL data model.

### Added
- `Netlist`, `Module`, `Port`, `Net`, `NetSlice`, `Instance` data classes.
- `BUILTIN_CELL_TYPES` registry: 27 cell types with `CellTypeSig`.
- `Direction`, `Level` enums.
- Full JSON round-trip: `to_dict`/`from_dict`/`to_json`/`from_json`/`to_json_str`/`from_json_str`. Schema version frozen at 0.1.0; major-version mismatch detection.
- `Netlist.stats()` returns cell-counts and totals.
- `validate_netlist(nl)` (also accessible as `nl.validate()`) implements rules R1-R7 and R11.

### Out of scope (v0.2.0)
- EDIF / BLIF importer + exporter.
- Single-driver per net (R8).
- Combinational-loop detection (R10).
- Streaming readers / writers for very large designs.
