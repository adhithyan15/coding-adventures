### Changed — load committed modality owners only in their assertions

- Keep the shared modality-manifest test hook responsible for deriving the
  complete real-lesson manifest, while lazily reading every committed owner
  through the existing anchored async route when an assertion first needs it.
  Both committed-owner assertions still share one read-only promise and compare
  the same complete data; no source, owner, or check is skipped.
- Remove cold-disk contention between the independent lesson and owner readers
  from the hook's unchanged 30-second budget. The committed-marker case keeps
  its 30-second budget, the end-to-end `--check` case keeps its 60-second budget,
  and the package's worker, heap, and 85% coverage gates are unchanged.
- A local isolated Windows file passed all 44 existing cases after this change.
  Full-suite Windows and macOS evidence remains required for #18941, #18751,
  #18852, and #18947; an isolated pass does not close those issues.
