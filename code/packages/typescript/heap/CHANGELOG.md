# Changelog

## [Unreleased]

### Fixed

- `BUILD_windows` now mirrors `BUILD` (`npm run build` + `npm run test:coverage`); it still ran `npx vitest`, which this package no longer depends on.

## 0.1.0

- Initial TypeScript port of the DT04 heap package
- Added `MinHeap`, `MaxHeap`, and pure heap helper functions
