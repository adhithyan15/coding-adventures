# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-05-06

### Added

- Thread role, MLE command, TLV, scan-mask, and mode primitives.
- MLE message parser/encoder and a deterministic parent/child attach-state
  skeleton for simulator-first Thread work.
- Typed Leader Data TLV helpers plus opaque Network Data extraction from MLE
  messages for diagnostics and simulator fixtures.
- Thread Network Data TLV parsing/encoding with stable-bit preservation and
  typed Prefix TLV projection for prefix diagnostics.
- Typed Connectivity TLV helpers for route-cost, link-quality, active-router,
  and sleepy-end-device diagnostic fields.
- Thread diagnostic snapshots that combine neighbor, leader, connectivity,
  partition, and prefix state for D23-facing health reads.
- Thread supervisor action projections that classify diagnostic drift into
  stable repair intents for runtime supervisors.
- Neighbor table summaries for cheap runtime/read-model projections of parent,
  child, router, stale-neighbor, and parent-candidate state.
- Neighbor table primitives for parent/child/router relationships, stale
  timeout expiry, link margin tracking, and parent-candidate selection.
