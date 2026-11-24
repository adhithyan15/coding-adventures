## [0.1.0] - UNRELEASED

### Changed (KILN04, collect)

- A shard whose build tool was killed with one step running (a census shard at its
  build-time cap) now reports `interrupted: a step started and was never
  accounted for (the build was stopped, or its supervisor died)` instead of `serial-violated: the counter reached N and N-1 sidecars`.
  The shape is exact: one number missing, the highest, and exactly one step in the
  shard with a trace and no sidecar. Any other mismatch is still `serial-violated`.
- In that shard an action with no step directory says `not run: the build stopped
  before this step`.
- Only the reason text changes: every record of the shard is still an unknown write
  set, so no action becomes emittable. Tests cover each shape.
