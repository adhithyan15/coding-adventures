# Changelog — Heap (Swift)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Fixed

- `BUILD_windows`: `where swift` now discards its output with cmd's `>nul
  2>nul` instead of `>/dev/null`. Under `cmd /C` the `/dev/null` redirect
  always failed, so the line always fell through to its "Swift not available"
  echo and the Swift step never ran on Windows, even on runners with Swift
  installed.

## [0.1.0] — 2026-04-12

### Added

- Initial implementation of `Heap`.
- Generic `MinHeap<Element>` and `MaxHeap<Element>` binary heaps.
- Default `Comparable` initializers plus custom comparator initializers.
- Sequence-based heapify initializers.
- XCTest coverage for min-heap ordering, max-heap ordering, heapify, and
  custom comparator support.
