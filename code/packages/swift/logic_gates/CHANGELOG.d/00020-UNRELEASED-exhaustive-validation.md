## [Unreleased]

### Added

- `GatesExhaustiveTests.swift`: literal truth tables for all 11 basic gates,
  NAND-built == direct, and rejection of every Int in -1024...1024 except
  0/1 plus Int.min/max (and neighbours) and Int32 bounds, in every argument
  position, asserting exactly `.invalidBit(name:got:)` with the first
  invalid parameter name.
- Documented (Gates.swift, README) the validation contract and that Swift's
  type system rejects Bool/Double/Int8/UInt arguments at compile time.

### Changed

- Existing bare `XCTAssertThrowsError` gate tests now assert the error case
  and payload. No behaviour change: every gate already validated its own
  arguments (a, then b) before computing.
