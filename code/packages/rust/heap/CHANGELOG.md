# Changelog

## Unreleased

- Added bounded min- and max-priority queues with logical capacity, fallible
  reservation, owned rejection errors, and bottom-up iterable construction.
  Preserved the unbounded APIs and source import name while renaming the Cargo
  package to `coding-adventures-heap` and aliasing direct dependents.
- Refreshed the three separate transitive Cargo lockfiles that still recorded
  the old package name, so standalone `--locked` consumers resolve the rename.
- Added independent sorted-vector model tests: short exhaustive histories,
  seeded replayable histories, deterministic byte histories, allocation
  capacity-overflow seam checks, and legacy helper regressions. These are
  bounded test campaigns, not a universal proof or a production coverage claim.

- Added the initial Rust `heap` crate with min-heap, max-heap, and helper algorithms.
