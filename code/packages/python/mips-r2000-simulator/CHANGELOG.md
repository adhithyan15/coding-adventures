# Changelog

## [0.1.0] — 2026-05-04

### Added

- Initial implementation of `MIPSSimulator` — behavioral simulator for the MIPS R2000 (1985)
- `MIPSState` frozen dataclass capturing complete CPU state (PC, 32 GPRs, HI, LO, 64 KB memory, halted flag)
- Full SIM00 protocol compliance: `reset()`, `load()`, `step()`, `execute()`, `get_state()`
- R-type instructions: SLL, SRL, SRA, SLLV, SRLV, SRAV, JR, JALR, MFHI, MTHI, MFLO, MTLO, MULT, MULTU, DIV, DIVU, ADD, ADDU, SUB, SUBU, AND, OR, XOR, NOR, SLT, SLTU
- REGIMM instructions: BLTZ, BGEZ, BLTZAL, BGEZAL
- I-type instructions: ADDI, ADDIU, SLTI, SLTIU, ANDI, ORI, XORI, LUI, LB, LH, LW, LBU, LHU, SB, SH, SW, BEQ, BNE, BLEZ, BGTZ
- J-type instructions: J, JAL
- HALT convention: SYSCALL (op=0, funct=0x0C) halts the simulator
- BREAK (funct=0x0D) raises `ValueError` (software breakpoint)
- Big-endian memory: LW/SW, LH/LHU/SH, LB/LBU/SB all use big-endian byte order
- Misaligned access detection: LW/SW require 4-byte alignment; LH/SH require 2-byte alignment
- Signed overflow detection: ADD, ADDI, SUB raise `ValueError` on overflow
- Division-by-zero detection: DIV and DIVU raise `ValueError`
- R0 always-zero invariant enforced in `_set_reg()`
- `max_steps` guard in `execute()` (default 100,000) prevents infinite loops
- 4 test modules: protocol compliance, per-instruction, end-to-end programs, edge cases
- Spec: `code/specs/07q-mips-r2000-simulator.md`
