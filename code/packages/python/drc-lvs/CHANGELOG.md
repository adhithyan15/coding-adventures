# Changelog

## [0.1.0] — Unreleased

### Added
- DRC: `Rect(layer, x1, y1, x2, y2)`, `Rule(name, layer, kind, value, severity)`, `Violation(...)`, `DrcReport`.
- DRC rule kinds: `min_width`, `min_spacing` (pairwise per layer), `min_area`.
- `run_drc(rects, rules) -> DrcReport`.
- LVS: `LvsCell(name, cell_type, pins)`, `LvsNetlist(cells)`, `LvsReport(matched, ...)`.
- `lvs(layout, schematic) -> LvsReport` via net-signature partition refinement.

### Out of scope (v0.2.0)
- DRC: enclosure, end-of-line, antenna, density.
- DRC R-tree for scale.
- LVS via full VF2 graph isomorphism.
- PEX (parasitic extraction).
- ERC (Electrical Rules Check).
