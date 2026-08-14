# Changelog

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
