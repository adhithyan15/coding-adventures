# Changelog

## 0.16.0

- Route structural relationships through deterministic orthogonal polylines when requested.

## 0.15.0

- Resolve explicit structural relationship ports to deterministic boundary anchors.

## 0.14.0

- Resolve structural group-edge modifiers to deterministic group-boundary endpoints.

## 0.13.0

- Carry structural start and end arrowhead intent into resolved relationship geometry.

## 0.12.0

- Reserve structural header geometry for Architecture service icon text.

## 0.11.0

- Resolve structural row and column alignment constraints to deterministic geometry.

## 0.10.0

- Resolve typed structural junctions to compact backend-neutral geometry.

## 0.9.0

- Reserve a deterministic title band in structural diagram geometry.

## 0.8.0

- Reserve structural node geometry from resolved typography styles.

## 0.7.0

- Resolve structural node styles into deterministic layout IR.

## 0.6.0

- Carry structural accessibility metadata into resolved IR.

## 0.5.0

- Carry typed Requirement definition kinds through structural layout inputs.

## 0.4.0

- Accept typed structural node metadata without changing resolved geometry.

## 0.3.0

- Honor explicit TB, BT, LR, and RL structural diagram directions.

## 0.1.0 — 2026-04-24

### Added
- Initial release as part of DG04 extended diagram families
- `layout_structural_diagram(diagram)` — lay out class/ER/C4 diagrams
- 3-column grid node placement with text-width-based node sizing
- Compartment height calculated from entry count
- Closest-side edge routing with dominant-axis selection
- 6 unit tests
