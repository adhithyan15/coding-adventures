# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- `Intel4004Simulator` -- standalone 4-bit accumulator-based processor simulation
- LDM, XCH, ADD, SUB, and HLT instructions
- 4-bit masking on all arithmetic operations
- Carry flag for overflow (ADD) and borrow (SUB) detection
- Step trace recording with accumulator and carry snapshots
- Encoding helpers for all supported instructions
