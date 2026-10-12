## [Unreleased]

- Repair #17335's quadratic styled Swimlane label run discovery with one monotone
  cursor per existing delimiter form and constant state. Preserve UTF-8 labels,
  first eligible terminating run, empty-label refusal, prefix-inside-run suffixes,
  authored precedence, kinds, markers and minimum lengths. Add actual parser work
  counters, retained red failures and exact semantic/work controls; no grammar,
  input/depth limit, dependency or duplicate-edge layout-index changes.
