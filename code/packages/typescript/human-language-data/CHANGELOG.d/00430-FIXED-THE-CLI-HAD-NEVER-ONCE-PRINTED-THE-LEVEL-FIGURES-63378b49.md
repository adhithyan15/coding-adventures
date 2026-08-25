### Fixed — the CLI had never once printed the level figures

- `report-cli` never passed `curricula` or `spine` to `buildCurriculumGapReport`, so
  the `levels` section has been silently absent from every CLI run since HL-C10
  shipped it. Both `levels` and the new gate now render. The section existed, was
  tested, and was invisible to anyone reading the report.

