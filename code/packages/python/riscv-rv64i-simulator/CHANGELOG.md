# Changelog

## 0.1.0 (2026-05-15)

Initial release — Layer 07y in the coding-adventures simulator series.

### Added

- `RV64ISimulator` implementing the SIM00 protocol:
  - `reset()`, `load()`, `step()`, `execute()`, `get_state()`
  - I/O stubs: `set_input_port()`, `get_output_port()`, `interrupt()`, `nmi()`
- `RV64IState` frozen dataclass with ABI register properties (ra, sp, gp, tp,
  t0–t6, s0–s11, a0–a7, zero) and full memory tuple
- `StepTrace` local dataclass with `pc_before`, `pc_after`, `halted`
- Full **RV64I** base integer instruction set:
  - LUI, AUIPC
  - JAL, JALR
  - BEQ, BNE, BLT, BGE, BLTU, BGEU
  - LB, LH, LW, LD, LBU, LHU, LWU
  - SB, SH, SW, SD
  - ADDI, SLTI, SLTIU, XORI, ORI, ANDI, SLLI, SRLI, SRAI
  - ADDIW, SLLIW, SRLIW, SRAIW
  - ADD, SUB, SLL, SLT, SLTU, XOR, SRL, SRA, OR, AND
  - ADDW, SUBW, SLLW, SRLW, SRAW
  - FENCE (NOP)
  - ECALL/EBREAK (halt)
- **M extension** multiply/divide:
  - MUL, MULH, MULHSU, MULHU, DIV, DIVU, REM, REMU (64-bit)
  - MULW, DIVW, DIVUW, REMW, REMUW (32-bit word, result sign-extended)
- Halt sentinel: 32-bit word `0x00000000`
- Reset initializes SP=0xFFF8, all other registers and memory = 0
- Full test suite: 15 protocol tests + instruction/program tests (>80% coverage)
- `py.typed` marker for PEP 561 compliance
- Literate code comments throughout
