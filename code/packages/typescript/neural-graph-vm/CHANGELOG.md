# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-04-29

### Added

- Added a reference Neural Graph VM package.
- Added a graph compiler that lowers `MultiDirectedGraph` metadata to NN00
  forward bytecode.
- Added a scalar bytecode interpreter for reference execution and smoke tests.
- Added neural primitive helpers for authoring input, weighted-sum, activation,
  and output graph nodes without manually constructing metadata.
