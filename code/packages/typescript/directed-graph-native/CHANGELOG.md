# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - 2026-03-21

### Added

- Initial release of `@coding-adventures/directed-graph-native`.
- napi-rs wrapper around the Rust `directed-graph` crate for Node.js.
- `DirectedGraph` class with full API: `addNode`, `removeNode`, `hasNode`, `nodes`,
  `addEdge`, `removeEdge`, `hasEdge`, `edges`, `predecessors`, `successors`,
  `size`, `edgeCount`, `toStringRepr`, `topologicalSort`, `hasCycle`,
  `transitiveClosure`, `affectedNodes`, `independentGroups`.
- TypeScript type definitions in `index.d.ts`.
- Error handling: `CycleError`, `NodeNotFoundError`, `EdgeNotFoundError`,
  `SelfLoopError` mapped to JavaScript `Error` with descriptive message prefixes.
- 45 Vitest tests mirroring the Python test suite, including a real repo
  integration test with 21 packages.
- BUILD file for the build system.
