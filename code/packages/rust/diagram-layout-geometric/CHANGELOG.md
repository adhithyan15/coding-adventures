# Changelog

## 0.1.0 — 2026-04-24

### Added
- Initial release as part of DG04 extended diagram families
- `layout_geometric_diagram(diagram)` — resolve canvas size for geometric diagrams
- Auto-size from element bounding boxes (Box, Circle, Line, Arc, Text)
- Respects explicit `width`/`height` overrides (partial or both)
- 7 unit tests

## Unreleased — bounded Venn canvas configuration

Preserve incoming canvas config and portable metadata while bounding source
scalars and adding a checked typed layout API. Recognize only direct config
members in closed single-line init objects; preserve whole-object front-matter
precedence and field defaults. Keep original supported geometry and every
unrelated production/test path. Add exact geometry and complete scene/query
controls; final affected 14-package strict lint and 918 unfiltered tests pass.
Final-source Miri DEFERRED/PENDING; hosted checks/merge/full parity remain OPEN.
