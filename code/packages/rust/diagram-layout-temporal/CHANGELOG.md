# Changelog

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
