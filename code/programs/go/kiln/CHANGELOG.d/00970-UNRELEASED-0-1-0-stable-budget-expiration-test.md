## [0.1.0] - UNRELEASED

### Fixed (verification budget test is deterministic on Windows)

- `TestVerifyTreeBudget` now uses an already-expired zero-duration budget. This
  keeps the test focused on the deadline error while avoiding host timer
  resolution races on Windows. Production budget behavior is unchanged.
