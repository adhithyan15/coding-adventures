# Changelog

All notable changes to the arithmetic package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-21

### Added
- Half adder built from XOR + AND gates
- Full adder built from two half adders + OR gate
- N-bit ripple carry adder chaining full adders
- ALU with 6 operations: ADD, SUB, AND, OR, XOR, NOT
- SUB uses two's complement via NOT + add with carry
- All operations route through logic_gates package primitives
- 11 tests covering all components
