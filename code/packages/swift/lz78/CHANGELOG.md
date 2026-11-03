# Changelog

## [Unreleased]

### Fixed

- `BUILD_windows`: `where swift` now discards its output with cmd's `>nul
  2>nul` instead of `>/dev/null`. Under `cmd /C` the `/dev/null` redirect
  always failed, so the line always fell through to its "Swift not available"
  echo and the Swift step never ran on Windows, even on runners with Swift
  installed.

## [0.1.0] - 2026-04-11

### Added

- `TrieCursor` struct — arena-based step-by-step trie cursor (exported for LZW reuse)
  - `step(_:)`, `insert(_:dictID:)`, `reset()`, `dictID`, `atRoot`, `entries()`
- `Token` struct — `(dictIndex: UInt16, nextChar: UInt8)`
- `encode(_:maxDictSize:)` — encode `[UInt8]` to LZ78 token array
- `decode(_:originalLength:)` — decode token array to `[UInt8]`
- `compress(_:maxDictSize:)` — one-shot compress with CMP01 wire format
- `decompress(_:)` — one-shot decompress
- `serialiseTokens(_:originalLength:)` / `deserialiseTokens(_:)`
- 33 tests covering spec vectors, round-trips, TrieCursor, wire format
