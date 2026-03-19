# Changelog

## 0.1.0 (2026-03-19)

### Added
- Initial TypeScript port from Python intel4004-simulator
- Intel4004Simulator: complete 4-bit accumulator-based processor simulation
- Instruction set: LDM, XCH, ADD, SUB, HLT
- 4-bit masking on all data values (0-15)
- Carry/borrow flag for arithmetic overflow detection
- Full test suite ported from Python with vitest
- Knuth-style literate programming comments preserved from Python source
