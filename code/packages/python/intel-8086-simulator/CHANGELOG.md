# Changelog — intel-8086-simulator

## 0.1.0 (2026-05-04)

### Added

- `X86State` — frozen dataclass snapshot of the complete 8086 CPU state:
  - 16-bit general-purpose registers AX, BX, CX, DX with 8-bit half access
    (AL/AH, BL/BH, CL/CH, DL/DH)
  - Index and pointer registers SI, DI, SP, BP
  - Segment registers CS, DS, SS, ES
  - Instruction pointer IP
  - Individual flag fields CF, PF, AF, ZF, SF, TF, IF, DF, OF as booleans
  - `flags` property returning packed 16-bit FLAGS register value
  - `ax_signed`, `al_signed` signed-integer views
  - `halted` field
  - 256-entry `input_ports` and `output_ports` tuples
  - 1,048,576-byte `memory` tuple (full 1 MB)

- `flags.py` — pure-function flag computation helpers:
  - `compute_cf_add`, `compute_cf_sub` — carry/borrow
  - `compute_af_add`, `compute_af_sub` — auxiliary carry
  - `compute_of_add`, `compute_of_sub` — signed overflow
  - `compute_szp` — sign, zero, parity from result
  - `compute_parity` — even-ones parity of low byte
  - `pack_flags`, `unpack_flags` — FLAGS register ↔ individual booleans

- `X86Simulator` implementing `Simulator[X86State]` (SIM00 protocol):
  - `reset()` — clears all registers, memory, flags, ports
  - `load(program, origin=0)` — writes bytes to physical memory
  - `step()` → `StepTrace` — one fetch-decode-execute cycle with real 8086
    instruction encoding (ModRM, displacement, immediate decode)
  - `execute(program, max_steps=10_000)` → `ExecutionResult[X86State]`
  - `get_state()` → frozen `X86State` snapshot

- Full instruction set coverage:
  - Data transfer: MOV (all forms), XCHG, PUSH/POP, PUSHF/POPF, LEA,
    LDS/LES, LAHF/SAHF, CBW, CWD, XLAT
  - Arithmetic: ADD, ADC, SUB, SBB, INC, DEC, NEG, CMP, MUL, IMUL,
    DIV, IDIV, DAA, DAS, AAA, AAS, AAM, AAD
  - Logical: AND, OR, XOR, NOT, TEST
  - Shifts/rotates: SHL/SAL, SHR, SAR, ROL, ROR, RCL, RCR (by 1 and by CL)
  - Control: JMP (short/near/far/indirect), CALL (near/far), RET, RETF,
    all 16 conditional jumps (Jcc), LOOP/LOOPE/LOOPNE, JCXZ, INT (halt),
    IRET
  - String: MOVS, CMPS, SCAS, LODS, STOS with REP/REPE/REPNE prefixes
  - Misc: NOP, HLT, CLC/STC/CMC, CLD/STD, CLI/STI, IN/OUT (byte/word,
    immediate/DX), LOCK (ignored), WAIT (ignored)
  - Segment override prefixes: ES:/CS:/SS:/DS:

- Three test suites, 100% line coverage:
  - `test_protocol.py` — SIM00 contract (construction, reset, load, step,
    execute with halt and max_steps, get_state snapshot isolation)
  - `test_instructions.py` — per-instruction tests including flag edge cases,
    ModRM addressing modes, 8-bit vs 16-bit variants
  - `test_programs.py` — multi-instruction programs: sum loop, factorial,
    string copy, GCD, bubble sort, I/O port roundtrip
