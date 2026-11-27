# Changelog

## Unreleased

- Added the dependency-free C17 fixed-geometry HDD simulator with exact timing,
  explicit buffer lengths, typed statuses, and detached snapshots.
- Added canonical trace replay, timing boundary probes, exhaustive timing
  states, and deterministic independent-model tests.
- Added a strict bounded trace adapter shared by native tests and separate
  fixture/history libFuzzer targets, with a tick-stepping full-media oracle,
  refusal sentinel checks, fixed seeds, and sanitizer/coverage receipts.
