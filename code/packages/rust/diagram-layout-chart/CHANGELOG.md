# Changelog

## 0.14.0

- Lay out inferred and explicit numeric Mermaid XY x-axes with evenly distributed series positions and numeric ticks.

## 0.13.0

- Resolve Mermaid XY line point labels into backend-neutral text bounds for vertical and horizontal charts.

## 0.12.0

- Lay out horizontal Mermaid XY charts with categorical rows, numeric columns, horizontal bars, and transposed line paths.

## 0.11.0

- Reserve XY-chart plot margins for axis titles and lower them to backend-neutral chart labels.

## 0.8.0

- Resolve quadrant region, label, point, axis, title, and independent border theme colors.

## 0.7.0 — 2026-08-13

### Added
- Resolve quadrant typography and label padding into chart layout items

## 0.6.0 — 2026-08-13

### Added
- Apply quadrant padding and emit one frame plus independent internal dividers

## 0.5.0 — 2026-08-13

### Added
- Apply authored quadrant dimensions, axis positions, and default point radius

## 0.4.0 — 2026-08-13

### Added
- Preserve chart accessibility metadata through layout

## 0.3.0 — 2026-08-13

### Added
- Resolve authored quadrant point radius and paint styles into scatter-point geometry

## 0.2.0 — 2026-08-13

### Added
- Native quadrant regions, endpoint labels, and normalized scatter-point layout

## 0.1.0 — 2026-04-24

### Added
- Initial release as part of DG04 extended diagram families
- `layout_chart_diagram(diagram, cw, ch)` — lay out XY, Pie, or Sankey charts
- XY chart: margin-based plot area, bar groups, line paths, grid lines, legend
- Pie chart: angular accumulation starting at 12 o'clock (−π/2)
- Sankey: proportional left-to-right horizontal bands
- 5 unit tests
