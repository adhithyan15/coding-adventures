# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-18

### Added
- Half adder (XOR + AND)
- Full adder (two half adders + OR)
- Ripple-carry adder for N-bit addition with overflow detection
- Ripple-carry adder with explicit carry-in (for subtraction support)
- ALU with six operations: ADD, SUB, AND, OR, XOR, NOT
- ALU status flags: zero, carry, negative, overflow
- Two's complement subtraction (A + NOT(B) + 1)
- Signed overflow detection
- Comprehensive doc comments with circuit diagrams and truth tables
- Inline unit tests and integration test files
- Ported from Python arithmetic package
