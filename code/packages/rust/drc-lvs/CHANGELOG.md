# Changelog — drc-lvs

## [0.1.0] — 2026-06-13

### Added
- `DrcRect`, `Rule`, `RuleKind`, `Violation`, `DrcReport` — DRC data model.
- `Rule::min_width()`, `Rule::min_spacing()`, `Rule::min_area()` — constructors.
- `run_drc()` — dispatches by layer and rule kind; O(n²) pairwise spacing checks.
- `DrcReport::clean()` — returns true if no error-severity violations.
- `LvsCell`, `LvsNetlist`, `LvsReport` — LVS data model.
- `lvs()` — bag-of-cell-signatures comparison; instance-name-agnostic.
- `net_signatures()`, `cell_signatures()` — partition-refinement helpers.
- 13 integration tests + 2 doc-tests.
