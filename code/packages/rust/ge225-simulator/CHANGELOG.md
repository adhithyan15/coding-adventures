# Changelog

All notable changes to the ge225-simulator Rust package will be documented in this file.

## [Unreleased]

### Changed
- Store GE-225 modification words in their documented reserved core locations.
- Reject modified, branch, multiword, card-reader, and block-move ranges outside
  installed memory instead of silently wrapping them.
- Make program loads, double loads, card reads, and overlapping block moves
  atomic with respect to range failures.
- Store the `SPB` instruction address in its X word and expose a checked program
  counter setter for programs placed above the modification words.
- Return a constructor error outside the documented 4K-through-16K installed
  memory range instead of panicking or attempting an oversized allocation.
- Select the unmodified `BRU` bank from its already-advanced P counter and the
  `SPB` bank from the instruction address, while retaining full 15-bit targets
  for modified `BRU` instructions.
- Bound the simplified card-reader abstraction to 27 words per record and 64
  queued records.
- Reject a non-branching final-memory-word step before execution so successful
  state inspection can never expose a P counter outside installed memory.
- Define the maximum negative `SAN 31` fill path without signed host overflow.
- Expand the package BUILD recipe to enforce tests, warning-free Clippy, and
  warning-free rustdoc.

### Added
- Integration regressions for X-word addressing, out-of-range effective
  addresses and branches, atomic loader/card/double-word behavior, `SPB`, and
  overlapping `MOY` copies.

## [0.1.0] - 2026-04-15

### Added
- Initial Rust GE-225 behavioral simulator package
- Documented opcode maps, helpers, and focused execution tests
