# Changelog — sky130-pdk

## [0.1.0] — 2026-06-13

### Added
- `ProcessMetadata` — Sky130A process parameters (feature size, V_DD, V_t, μC_ox, metal layers, cell-row height).
- `LayerInfo` + `LAYER_MAP` — 23-entry GDS layer/datatype map covering nwell through met5 (drawing + pin + via layers).
- `CellInfo` + `TEACHING_CELLS` — 35-cell teaching subset of the Sky130 HD standard-cell library.
- `Pdk` struct with `get_cell()`, `get_layer()`, `cell_names()` accessors.
- `PdkProfile` enum: Teaching (in-memory) and Full (validates root path).
- `load_sky130()` function with `PdkError::MissingRoot` and `PdkError::InstallNotFound`.
- 12 integration tests + 3 doc-tests.
