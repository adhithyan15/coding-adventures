# Changelog

All notable changes to the intel4004_simulator package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-03-21

### Added
- Complete Intel 4004 simulator with all 46 real instructions + HLT
- Immutable struct-based CPU state (idiomatic Elixir)
- All instruction categories:
  - Data movement: LDM, LD, XCH, FIM, SRC, FIN, JIN
  - Arithmetic: ADD, SUB, INC, ADM, SBM (with MCS-4 complement-add semantics)
  - Accumulator ops: CLB, CLC, IAC, CMC, CMA, RAL, RAR, TCC, DAC, TCS, STC, DAA, KBP, DCL
  - Control flow: NOP, HLT, JUN, JCN, JMS, BBL, ISZ
  - I/O: WRM, WMP, WRR, WPM, WR0-3, RDM, RDR, RD0-3
- 3-level hardware call stack with mod-3 wrapping
- 4-bank × 4-register × 16-character RAM with status nibbles
- ROM I/O port simulation
- Execution tracing with before/after state
- 52 tests covering all instructions and edge cases
