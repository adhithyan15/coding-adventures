# Changelog — asic-routing

## [0.1.0] — 2026-06-13

### Added
- `PinAccess`, `RouteOptions`, `RouteReport`, `RouteError` — routing data model.
- `route()` — builds blocked grid from placed components, runs Lee BFS per net-pin pair, marks routed paths blocked, appends `Segment` records to the `Def`.
- `lee_maze_route()`, `reconstruct_path()`, `path_to_segment()`, `segment_length()` helpers.
- `to_grid()`, `pin_at()` — coordinate helpers.
- 9 integration tests + 1 doc-test.
