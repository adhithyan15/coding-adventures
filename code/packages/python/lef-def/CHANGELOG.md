# Changelog

## [0.1.0] — Unreleased

### Added
- Data classes: `TechLef`, `LayerDef`, `ViaDef`, `ViaLayer`, `SiteDef`, `CellLef`, `PinDef`, `PinPort`, `Rect`; `Def`, `Row`, `Component`, `DefPin`, `Net`, `Segment`; `Direction` and `Use` enums.
- `write_tech_lef(tech, path)` / `write_tech_lef_str(tech)`.
- `write_cells_lef(cells, path)` / `write_cells_lef_str(cells)`.
- `write_def(def_obj, path)` / `write_def_str(def_obj)`.
- LEF emission: VERSION/UNITS, LAYER (TYPE/DIRECTION/PITCH/WIDTH/SPACING), VIA (with multi-layer rects), SITE; MACRO (CLASS/SIZE/SITE/PIN/OBS) with PORT layer/rect listings.
- DEF emission: VERSION/UNITS, DIEAREA, ROW, COMPONENTS (placed `+ PLACED ( x y ) orient`), PINS (with optional layer/rect), NETS (with optional ROUTED segments per-layer).

### Out of scope (v0.2.0)
- LEF/DEF parsers.
- SPECIALNETS, GROUPS, REGIONS, BLOCKAGES, TRACKS.
- LEF 5.9 features (NDR rules, antenna properties, etc.).
