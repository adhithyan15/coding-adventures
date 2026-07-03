# Changelog — lef-def

## [0.1.0] — 2026-06-13

### Added
- `TechLef`, `CellLef`, `Def`, `Net`, `Component`, `Row`, `Segment`, `DefPin`, `PinPort`, `PinDef`, `Rect`, `LayerDef`, `ViaDef`, `SiteDef`, `Direction`, `Use` — full LEF/DEF object model.
- `write_tech_lef_str()` — technology LEF emission with LAYER, VIA, SITE sections.
- `write_cells_lef_str()` — cell LEF emission with MACRO, PIN, OBS sections.
- `write_def_str()` — DEF 5.8 emission with DIEAREA, ROW, COMPONENTS, PINS, NETS sections including ROUTED geometry.
- `Component::new()`, `Net::new()`, `Def::new()` — convenience constructors.
- 14 integration tests.
