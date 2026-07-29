# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-29

### Added

- Added a generic multi-directed graph with stable edge IDs and parallel edges.
- Added graph, node, and edge property bags with synchronized edge weights.
- Added deterministic predecessor, successor, topological sort, cycle, and
  independent-group operations that account for edge multiplicity.
- Added `GraphError` variants for missing nodes, missing edges, duplicate IDs,
  invalid weights, cycles, and disallowed self-loops.
