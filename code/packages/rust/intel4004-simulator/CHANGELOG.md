# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-03-21

### Added

- All 46 Intel 4004 instructions (up from 5 in MVP)
- NOP, LD, INC, BBL instructions for basic operations
- JCN (conditional jump), JUN (unconditional jump), JMS (subroutine call) for control flow
- ISZ (increment and skip if zero) for loop construction
- FIM (fetch immediate to register pair), SRC (send register control) for memory addressing
- FIN (fetch indirect from ROM), JIN (jump indirect) for indirect operations
- Full I/O subsystem: WRM, RDM, WMP, WRR, RDR, ADM, SBM, WR0-WR3, RD0-RD3, WPM
- Accumulator group: CLB, CLC, IAC, CMC, CMA, RAL, RAR, TCC, DAC, TCS, STC, DAA, KBP, DCL
- Hardware 3-level subroutine stack with push/pop and wrapping behavior
- Data RAM: 4 banks x 4 registers x 16 characters (nibbles)
- RAM status characters: 4 banks x 4 registers x 4 status nibbles
- RAM output port (one 4-bit latch per bank)
- ROM I/O port (4-bit)
- RAM bank selection via DCL instruction
- Register pair helpers (read_pair, write_pair) for 8-bit pair operations
- 2-byte instruction detection and fetch in step()
- `raw2` field on Intel4004Trace for 2-byte instruction recording
- `reset()` method to zero all CPU state
- `run()` now calls `reset()` before loading a new program
- Encoding helpers for all 46 instructions
- 82 comprehensive tests covering all instructions, stack ops, RAM, BCD, KBP, and end-to-end programs

### Changed

- ADD now includes carry in the addition (A + R + carry), matching real 4004 behavior
- SUB now uses complement-and-add method with borrow propagation, matching real 4004 behavior
- `execute()` signature expanded to accept raw byte, optional second byte, and instruction address
- `step()` now detects and fetches 2-byte instructions before executing

## [0.1.0] - 2026-03-19

### Added

- `Intel4004Simulator` -- standalone 4-bit accumulator-based processor simulation
- LDM, XCH, ADD, SUB, and HLT instructions
- 4-bit masking on all arithmetic operations
- Carry flag for overflow (ADD) and borrow (SUB) detection
- Step trace recording with accumulator and carry snapshots
- Encoding helpers for all supported instructions
