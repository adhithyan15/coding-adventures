# Changelog

All notable changes to `@coding-adventures/forme-authoring-preview` are recorded
here.

## 0.1.0 — 2026-10-03

### Added

- Added an exact-revision coordinator that snapshots validated durable
  authoring state and delegates isolated project materialization to a narrow
  host boundary.
- Routed every prepared revision through the existing FM03
  `Orchestrator.watch` path and FM07 static-artifact conversion rather than a
  second renderer.
- Added bounded edit coalescing, superseded preparation and build
  cancellation, idempotent cleanup, and disposal.
- Preserved the last good artifact set across failed, cancelled, malformed, or
  superseded builds while reporting active and last-good revisions separately.
- Added closed, capped, redacted diagnostic records and defensive handling for
  hostile or malformed host results.
- Added a generation-bound synchronous publisher commit gate, bounded immutable
  artifact snapshots, exact identifier rejection, and shared concurrent
  disposal so stale or malformed host activity fails closed.
- Hardened production `Session` accessor snapshots, typed-array intrinsic
  sizing/copying, portable path collisions, proxy rejection, watch-retirement
  failures, publisher abort races, and commit-time last-good attribution.
- Added adversarial lifecycle tests with 95%+ statement, 99%+ line, 100%
  function, and 90%+ branch coverage.
