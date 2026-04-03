# Changelog — coding-adventures-intel4004-simulator (Lua)

## 0.1.0 — 2026-03-31

Initial release. Lua port of the Elixir intel4004_simulator package.

### Added

- Complete Intel 4004 instruction set (46 instructions + HLT)
- 4-bit accumulator architecture with carry flag
- 16 × 4-bit registers (R0-R15) organized as 8 pairs (P0-P7)
- 12-bit program counter (4096-byte ROM)
- 3-level hardware stack for JMS/BBL subroutine calls
- RAM: 4 banks × 4 registers × 16 main characters + 4 status characters
- ROM I/O port (WRR/RDR)
- Instruction tracing (address, mnemonic, accumulator before/after)
- BCD arithmetic instructions (DAA, TCS)
- Comprehensive test suite covering all instruction categories and E2E programs
