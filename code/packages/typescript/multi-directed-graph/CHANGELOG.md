# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Fixed

- `BUILD_windows` now mirrors `BUILD` (`npm run build` + `npm run test:coverage`); it still ran `npx vitest`, which this package no longer depends on.

## [0.1.0] - 2026-04-29

### Added

- Added `MultiDirectedGraph` with stable edge IDs and parallel directed edges.
- Added graph, node, and edge property bags using JSON-like scalar values.
- Added canonical edge weights synchronized with the `weight` edge property.
- Added successor/predecessor, incoming/outgoing edge, topological sort, cycle,
  and independent group helpers.
