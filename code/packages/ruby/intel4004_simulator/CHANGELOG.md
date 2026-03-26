# Changelog

## [0.2.0] - 2026-03-21

### Added
- Complete Intel 4004 instruction set: all 46 real instructions now implemented
- NOP (0x00): no operation
- JCN (0x1_): conditional jump with 4-bit condition code (invert, test_zero, test_carry, test_pin)
- FIM (0x2_ even): fetch immediate 8-bit data to register pair
- SRC (0x2_ odd): send register control (set RAM/ROM address for I/O)
- FIN (0x3_ even): fetch indirect from ROM via register pair P0
- JIN (0x3_ odd): jump indirect via register pair
- JUN (0x4_): unconditional 12-bit jump
- JMS (0x5_): jump to subroutine (push return address to 3-level hardware stack)
- INC (0x6_): increment register (no carry effect)
- ISZ (0x7_): increment and skip if zero (loop counter)
- LD (0xA_): load register into accumulator
- BBL (0xC_): branch back and load (return from subroutine with immediate value)
- I/O instructions: WRM, WMP, WRR, WPM, WR0-WR3, SBM, RDM, RDR, ADM, RD0-RD3
- Accumulator ops: CLB, CLC, IAC, CMC, CMA, RAL, RAR, TCC, DAC, TCS, STC, DAA, KBP, DCL
- 3-level hardware call stack with silent wrap on overflow
- Full RAM model: 4 banks x 4 registers x (16 main + 4 status) nibbles
- RAM output ports (one per bank, written by WMP)
- ROM I/O port (written by WRR, read by RDR)
- RAM bank selection via DCL instruction
- RAM register/character addressing via SRC instruction
- Register pair read/write helpers
- `reset` method to restore all CPU state to power-on defaults
- `halted?` predicate method
- `raw2` field on Intel4004Trace for 2-byte instruction tracing
- 2-byte instruction detection and proper PC advancement
- Comprehensive literate programming documentation explaining hardware concepts

### Changed
- SUB now uses correct complement-add semantics: A + ~Rn + borrow_in (carry=true means NO borrow)
- ADD now includes carry flag in computation (A + Rn + carry)
- Trace structure extended with `raw2` field for 2-byte instructions
- `run` method now calls `reset` before loading new program

## [0.1.0] - 2026-03-18

### Added
- Intel4004Sim: standalone 4-bit accumulator machine simulator
- Instructions: LDM (load immediate), XCH (exchange), ADD, SUB, HLT
- 4-bit value masking, carry/borrow flag tracking
- Intel4004Trace: immutable Data.define trace records
