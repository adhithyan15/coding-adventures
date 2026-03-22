# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-03-21

### Added

- All 46 real Intel 4004 instructions (up from 5 MVP instructions)
- 3-level hardware call stack with mod-3 wrapping (`hw_stack`, `stack_pointer`)
- RAM subsystem: 4 banks x 4 registers x 16 main nibbles + 4 status nibbles
- RAM output ports (one per bank)
- ROM I/O port
- RAM addressing via SRC instruction (`ram_bank`, `ram_register`, `ram_character`)
- Jump instructions: JUN, JMS, JCN, JIN, ISZ (with 2-byte instruction support)
- Subroutine support: JMS (call), BBL (return with immediate load)
- Register pair operations: FIM (load pair), SRC (send address), FIN (indirect ROM read)
- Register operations: LD (load to accumulator), INC (increment), ISZ (loop counter)
- I/O instructions: WRM, WMP, WRR, WPM, WR0-WR3, SBM, RDM, RDR, ADM, RD0-RD3
- Accumulator operations: CLB, CLC, IAC, CMC, CMA, RAL, RAR, TCC, DAC, TCS, STC, DAA, KBP, DCL
- NOP instruction
- `Intel4004Trace.raw2` field for 2-byte instruction tracing
- `reset()` method to clear all CPU state
- Encoding helpers for all 46 instructions
- 75 unit tests covering every instruction plus integration tests
- Literate programming documentation throughout

### Changed

- `registers` field changed from `Vec<u8>` to `[u8; 16]` (fixed-size array)
- SUB now uses correct complement-add semantics: `A + (~Rn & 0xF) + borrow_in`
  where carry=true means NO borrow (matching MCS-4 manual)
- ADD now includes carry flag in sum: `A = A + Rn + carry`
- `run()` now calls `reset()` before loading the program
- `step()` handles 2-byte instruction fetch internally

## [0.1.0] - 2026-03-19

### Added

- `Intel4004Simulator` -- standalone 4-bit accumulator-based processor simulation
- LDM, XCH, ADD, SUB, and HLT instructions
- 4-bit masking on all arithmetic operations
- Carry flag for overflow (ADD) and borrow (SUB) detection
- Step trace recording with accumulator and carry snapshots
- Encoding helpers for all supported instructions
