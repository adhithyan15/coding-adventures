# Changelog

All notable changes to `@coding-adventures/forme-authoring-publish` are recorded
here.

## 0.1.0 — 2026-10-04

### Added

- Added an exact-revision authoring publication coordinator over the existing
  FM08 manifest, verified content reader, and reviewed target boundary.
- Added closed target review data, canonical manifest identity attribution,
  full content preflight, and exact-revision durable acknowledgement.
- Added fail-closed cancellation, overlap rejection, idempotent preparation
  retirement, redacted diagnostics, and poisoned retry after uncertain
  external, cleanup, or persistence outcomes.
- Added hostile-input and lifecycle tests with 97%+ statements, 99%+ lines,
  100% functions, and 90%+ branch coverage.
- Hardened synchronous adapter re-entry and replaced the raw builder store with
  a revocable manifest-restricted wrapper that re-verifies every target read.
- Reserved publication admission before session inspection, rejected proxy and
  accessor-backed boundaries with bounded schema snapshots, and made concurrent
  disposal share one cleanup settlement.
- Reused the core's exact opaque revision validator across snapshot, staleness,
  and durable-record phases so a deployed token cannot fail only at persistence.
- Treated resolved preparations without a capturable own retirement method as
  indeterminate cleanup and poisoned retry instead of allowing resource overlap.
