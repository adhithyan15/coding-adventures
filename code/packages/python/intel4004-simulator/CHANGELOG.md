# Changelog

All notable changes to the intel4004-simulator package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-03-21

### Changed
- **BREAKING:** Complete rewrite using GenericVM from virtual-machine package as execution engine
- Dependency changed from `coding-adventures-cpu-simulator` to `coding-adventures-virtual-machine`
- `Intel4004Simulator` now uses opcode handler registration instead of custom decode/execute loop

### Added
- Complete Intel 4004 instruction set — all 46 real instructions:
  - NOP, HLT (simulator-only halt)
  - Register ops: LD, XCH, INC
  - Arithmetic: ADD, SUB, ADM, SBM (with carry semantics matching MCS-4 manual)
  - Accumulator ops: CLB, CLC, IAC, CMC, CMA, RAL, RAR, TCC, DAC, TCS, STC, DAA, KBP, DCL
  - Jump/branch: JUN, JCN, ISZ, JMS, BBL
  - Register pair: FIM, SRC, FIN, JIN
  - RAM I/O: WRM, WMP, WRR, WPM, WR0–WR3, RDM, RDR, RD0–RD3
- Full RAM model: 4 banks × 4 registers × (16 main + 4 status) nibbles
- 3-level hardware call stack with silent overflow (wraps mod 3)
- RAM banking via DCL instruction
- RAM addressing via SRC instruction
- ROM I/O port (WRR/RDR)
- `Intel4004Trace` dataclass for execution tracing (address, raw bytes, mnemonic, before/after state)
- `load_program()` and `step()` methods for interactive debugging
- `reset()` method to clear all CPU state
- 112 tests covering all instructions, edge cases, and end-to-end programs
- 97%+ test coverage

## [0.1.0] - 2026-03-18

### Added
- Initial package scaffolding with pyproject.toml, src layout, and test structure
- MVP with 5 instructions: LDM, XCH, ADD, SUB, HLT
