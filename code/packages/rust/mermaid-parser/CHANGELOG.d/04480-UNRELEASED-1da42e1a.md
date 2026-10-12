## [Unreleased]

- Repair #17335's quadratic styled Swimlane label run discovery with one monotone
  cursor per existing delimiter form and constant state. Preserve UTF-8 labels,
  first eligible terminating run, empty-label refusal, prefix-inside-run suffixes,
  authored precedence, kinds, markers and minimum lengths. Add actual parser work
  counters, retained red failures and exact semantic/work controls; no grammar,
  input/depth limit, dependency or duplicate-edge layout-index changes.

- Reconcile upstream edge identifiers without restoring suffix rescans. Repair
  #17343 by advancing identifier discovery over the complete UTF-8 whitespace
  scalar. Public controls compare exact edge metadata for ASCII, two-byte and
  three-byte separators, standard/styled operators, chained IDs and absent IDs.
- Correct this shard's filename using its exact heading digest; the original
  required CI document-reconstruction failure and native panic red are retained.
