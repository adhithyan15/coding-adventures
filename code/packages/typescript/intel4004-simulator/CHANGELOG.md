# Changelog

## 0.2.0 (2026-03-21)

### Added
- Complete Intel 4004 instruction set: all 46 real instructions implemented
- Jump instructions: JUN, JCN (conditional), JIN (indirect), ISZ (loop counter)
- Subroutine support: JMS (call) and BBL (return with value)
- Register pair instructions: FIM (load pair), SRC (set RAM address), FIN (indirect ROM read)
- Register operations: LD (load), INC (increment)
- RAM I/O: WRM, RDM, WR0-WR3, RD0-RD3, SBM, ADM, WMP, WRR, WPM, RDR
- Accumulator operations: CLB, CLC, IAC, CMC, CMA, RAL, RAR, TCC, DAC, TCS, STC, DAA, KBP, DCL
- NOP instruction
- 3-level hardware call stack with mod-3 wraparound
- RAM: 4 banks x 4 registers x (16 main + 4 status) nibbles
- ROM I/O port
- RAM output ports (one per bank)
- `raw2` field in Intel4004Trace for 2-byte instructions
- `reset()` method for full CPU state reset
- 108 comprehensive tests covering all instructions

### Changed
- ADD now includes carry flag in computation (A = A + Rn + carry), matching the real 4004
- SUB now uses complement-add (A = A + ~Rn + borrow_in), where carry=true means NO borrow
- `run()` now calls `reset()` before loading program, ensuring clean state

## 0.1.0 (2026-03-19)

### Added
- Initial TypeScript port from Python intel4004-simulator
- Intel4004Simulator: complete 4-bit accumulator-based processor simulation
- Instruction set: LDM, XCH, ADD, SUB, HLT
- 4-bit masking on all data values (0-15)
- Carry/borrow flag for arithmetic overflow detection
- Full test suite ported from Python with vitest
- Knuth-style literate programming comments preserved from Python source
