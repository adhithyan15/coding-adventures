# Changelog

## 0.34.0

- Resolve typed Gantt quarter dates to their first month before backend-neutral layout.

## 0.33.0

- Resolve typed unpadded Gantt time fields into backend-neutral temporal geometry.

## 0.32.0

- Validate typed numeric and two-letter Gantt weekdays before backend-neutral layout.

## 0.31.0

- Resolve and validate typed Gantt ordinal days before backend-neutral temporal layout.

## 0.30.0

- Resolve typed one-, two-, and three-digit Gantt fractional seconds into precise backend-neutral geometry.

## 0.29.0

- Resolve and validate typed English weekday names before backend-neutral Gantt layout.

## 0.28.0

- Resolve typed 12-hour Gantt clocks into backend-neutral temporal geometry.

## 0.27.0

- Resolve single-component Gantt second timestamps into precise backend-neutral task geometry.

## 0.26.0

- Resolve typed Gantt timezone offsets into UTC-normalized backend-neutral task geometry.

## 0.25.0

- Reserve backend-neutral row geometry for multiline Mermaid Gantt titles, sections, and task labels.

## 0.24.0

- Resolve typed sub-day Gantt durations at millisecond precision before backend-neutral temporal layout.

## 0.23.0

- Resolve typed Gantt calendar, textual-month, time-of-day, and Unix timestamp input formats into backend-neutral temporal geometry.

## 0.22.0

- Preserve combined Gantt task tags through layout and lower `vert` tasks to full-height markers without consuming task rows.

## 0.21.0

- Resolve chained and multi-source Gantt starts to the latest dependency end and `until` ranges to the earliest referenced start.

## 0.20.0

- Apply `inclusiveEndDates` to explicit Gantt end dates and place the standard bottom axis plus optional top axis.
- Resolve the current-day marker and supported inline stroke declarations before backend-neutral Paint lowering.

## 0.19.0

- Extend Gantt task bars and dependency starts across excluded calendar days, honoring explicit includes and weekend boundaries.
- Resolve configured Gantt axis formats and tick intervals into backend-neutral time-axis labels.

## 0.16.0

- Route GitGraph merge and cherry-pick arcs from resolved source commits instead of event-order guesses.

## 0.15.0

- Lay out GitGraph lanes, commits, and merge arcs in LR, TB, and BT directions.

## 0.14.0

- Resolve GitGraph commit types and event kinds into backend-neutral commit symbols.

## 0.13.0

- Resolve GitGraph branch lanes with Mermaid 11.16.1 explicit and implicit ordering semantics.
- Emit branch labels in deterministic lane order for backend-neutral paint lowering.

## 0.12.0

- Preserve ordered GitGraph tag lists on temporal commit nodes.

## 0.11.0

- Reserve a backend-neutral GitGraph title row and carry accessibility metadata into temporal layout IR.

## 0.10.0

- Lay out Journey sections and tasks horizontally with score-ranked faces and activity lines.

## 0.9.0

- Resolve Journey task offsets and deterministically wrap actor legends to configured bounds.

## 0.8.0

- Resolve cyclic Journey actor, section fill, and section text palettes.

## 0.7.0

- Resolve styled Journey titles into dedicated backend-neutral layout items.

## 0.6.0

- Resolve Journey task typography onto backend-neutral temporal layout items.

## 0.5.0

- Resolve Journey margins, task dimensions, and task spacing from Mermaid init configuration.

## 0.4.0

- Resolve sorted Journey actor legends and deterministic actor colors.

## 0.3.0

- Reserve dynamic Journey row heights for normalized multiline labels.

## 0.2.0

- Lay out Mermaid user-journey sections and scored task rows.

## 0.1.0 — 2026-04-24

### Added
- Initial release as part of DG04 extended diagram families
- `layout_temporal_diagram(diagram, cw)` — lay out Gantt or git-graph
- Gantt: two-pass date resolution (absolute dates + `after <id>` deps)
- Gantt: weekly axis ticks, section headers, task bars, milestone markers
- Git-graph: branch lane assignment, commit nodes, merge arcs
- 7 unit tests
