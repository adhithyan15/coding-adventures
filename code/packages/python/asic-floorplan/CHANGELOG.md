# Changelog

## [0.1.0] — Unreleased

### Added
- `CellInstanceEstimate(instance_name, cell_type, area)`: cell needing placement.
- `IoSpec(name, direction, use)`: top-level IO pin to be placed on the die boundary.
- `Floorplan` dataclass: computed die rect, core rect, rows, unplaced components, pins.
- `compute_floorplan(cells, site_height, site_width, site_name, utilization=0.7, aspect=1.0, io_ring_width=10.0, io_pins=None, pin_layer="met2", design_name="design")`:
  - Sums cell area; divides by utilization for core area.
  - Computes core_height × core_width per the requested aspect ratio.
  - Snaps to integer site count and integer row count.
  - Generates alternating N/FS-oriented ROWs.
  - Distributes IO pins on edges (inputs left, outputs right, others bottom).
- `floorplan_to_def(fp, design_name)` -> `lef_def.Def`.

### Out of scope (v0.2.0)
- Power-grid generation (VDD/VSS rings + straps; needs SPECIALNETS in lef-def).
- Macro placement.
- Clock distribution.
- Iterative floorplan optimization.
