# Changelog

All notable changes to the ge225-simulator package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2026-04-19

### Fixed
- `_execute_branch_test`: the conditional was inverted (`if not cond: skip`) instead of
  the correct GE-225 semantics (`if cond: skip`). All skip-test instructions (BZE, BNZ,
  BMI, BPL, BOD, BEV, BOV, BNO, BPE, BPC, BNR, BNN) now correctly skip the next word
  when their named condition is **true**, matching the historical GE-225 programming manual.
  The bug had no observable effect on the existing tests because the affected branch tests
  coincidentally produced the same PC values by different code paths.

## [0.1.0] - 2026-04-15

### Added
- Initial implementation of a Python GE-225 behavioral simulator package
- 20-bit word-addressed memory model
- Real GE-225 mnemonic-oriented instruction registry based on documented octal forms
- Frozen `GE225State` and `GE225Indicators` snapshots
- `encode_instruction()` and `decode_instruction()` helpers for base memory-reference words
- `assemble_fixed()` and `assemble_shift()` helpers for documented fixed/shift commands
- `pack_words()` and `unpack_words()` helpers for protocol-friendly byte execution
- Initial execution support for arithmetic, transfer, compare, branch, and shift families
- `execute()` method returning `ExecutionResult[GE225State]`
- Test coverage for encoding, execution, branches, and protocol-style execution

### Changed
- Replaced the provisional backend-centric execution model with historically named GE-225 instruction handling
- Corrected central-processor semantics for skip-style `BXL`/`BXH`, `SPB`, `STO`, odd-address double-word operations, and block move `MOY`
- Expanded machine snapshots to expose `N`, index-group state, overflow/parity, decimal mode, and typewriter/control-switch status
- Added console/typewriter execution support for `RCS`, `TON`, `TYP`, `OFF`, `HPT`, `BNR`, and `BNN`
- Added package-local test path setup so the simulator can be validated without a separate editable install step
