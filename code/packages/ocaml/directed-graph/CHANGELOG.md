# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-09-05

### Added

- Added generic directed structure with synchronized forward and reverse maps.
- Added deterministic traversal, topological ordering, independent groups,
  closure, reverse dependents, affected sets, and strongly connected
  components.
- Added typed self-loop, cycle, weight, node, edge, and label failures with
  failure-atomic mutations.
- Added idempotent multi-label edges with label-specific and structural
  removal.
- Added Alcotest coverage for directed independence, property synchronization,
  cyclic and acyclic analysis, labels, and 1,000-node traversal.
