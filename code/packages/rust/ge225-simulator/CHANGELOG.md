# Changelog

All notable changes to the ge225-simulator Rust package will be documented in this file.

## [Unreleased]

### Changed
- Execute `ADD`, `SUB`, `DAD`, `DSU`, `ADO`, and `SBO` as three-digit BCD
  arithmetic while decimal mode is selected, including ten's-complement signs,
  end-of-field overflow, and carried or borrowed lower fields.
- Use the corrected manual's canonical `MOV` mnemonic for opcode 24 instead of
  the inherited `MOY` spelling.
- Decode `SXG` as the corrected `2506YY3` instruction and select its encoded
  five-bit Y group instead of deriving a group from A.
- Apply core-resident X words to fixed and shift instruction operands, reject
  modified shifts above 31 places before state changes, and expose explicit
  modified-instruction assembly helpers.
- Put the modified operand in the architectural I register and reject invalid
  effective addresses and branch targets before changing simulator state.
- Keep single-length `ADD`, `SUB`, `NEG`, `ADO`, `SBO`, and `SLA` overflow
  latched until `BOV` or `BNO`, and fail N-input shifts closed while N is not
  ready.
- Model A/Q double-length numbers as the documented one sign plus 38 data bits,
  with Q's sign duplicated or ignored according to the instruction, instead of
  treating the registers as a conventional 40-bit host integer.
- Correct `DAD`, `DSU`, `DCB`, `MPY`, `DVD`, `SRD`, and zero-count double-shift
  behavior against the October 1963 manual, including non-mutating divide
  overflow for zero and oversized divisors.
- Make `NOR` and `DNO` store their remainder in absolute memory location 0000
  regardless of the selected modification-word group.
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
- Deterministic 19-bit real-time-clock state and sixth-second host controls,
  with optional `LAC` and `LCA` instruction support and 24-hour wrapping.
- Corrected-manual vectors for positive and ten's-complement single/double
  decimal arithmetic, carry propagation, flagged-field overflow, invalid BCD,
  and real-time-clock transfer behavior, raising line coverage to 83.44%
  (902/1,081) after the new instruction families.
- Corrected-manual vectors for SXG group 27, fixed and shift automatic
  modification, the shift limit, single overflow latching, compare skips, and
  N-register readiness, bringing core line coverage to 82.64% (719/870).
- Manual example vectors for double add/subtract, multiply/divide, and
  double-register shifts, plus overflow, duplicated-sign, normalization, and
  zero-count regressions.
- Integration regressions for X-word addressing, out-of-range effective
  addresses and branches, atomic loader/card/double-word behavior, `SPB`, and
  overlapping `MOV` copies.

## [0.1.0] - 2026-04-15

### Added
- Initial Rust GE-225 behavioral simulator package
- Documented opcode maps, helpers, and focused execution tests
