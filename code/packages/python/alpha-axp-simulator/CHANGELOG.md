# Changelog — coding-adventures-alpha-axp-simulator

## [0.1.0] — 2026-05-05

Initial release — Layer 07s in the historical CPU simulator series.

### Added

- **`AlphaState`** frozen dataclass: `pc`, `npc`, `regs` (32 × 64-bit tuple), `memory`
  (65536-byte tuple), `halted`; convenience properties `.r0`–`.r31`, `.ra`, `.sp`,
  `.gp`, `.pv`, `.zero`
- **`AlphaSimulator`** implementing `Simulator[AlphaState]` (SIM00 protocol):
  - `reset()` — zeroes all state, PC=0, nPC=4
  - `load(data)` — reset + copy bytes to memory; raises `ValueError` if > 64 KiB
  - `step()` → `StepTrace` — execute one instruction
  - `execute(data, max_steps)` → `ExecutionResult` — load and run to HALT
  - `get_state()` → `AlphaState` — frozen snapshot

#### Instruction groups implemented

| Group | Opcode | Instructions |
|-------|--------|-------------|
| PALcode | 0x00 | HALT (call_pal 0x0000) |
| INTA | 0x10 | ADDL, ADDQ, SUBL, SUBQ, MULL, MULQ, CMPEQ, CMPLT, CMPLE, CMPULT, CMPULE, S4ADDL/Q, S8ADDL/Q, S4SUBL/Q, S8SUBL/Q |
| INTL | 0x11 | AND, BIC, BIS, ORNOT, XOR, EQV; CMOVLBS, CMOVLBC, CMOVEQ, CMOVNE, CMOVLT, CMOVGE, CMOVLE, CMOVGT; AMASK, IMPLVER |
| INTS | 0x12 | SLL, SRL, SRA; EXTBL/WL/LL/QL; INSBL/WL/LL/QL; MSKBL/WL/LL/QL; ZAP, ZAPNOT; SEXTB, SEXTW |
| INTM | 0x13 | MULL, MULQ, UMULH |
| Memory | various | LDL, LDQ, LDL_L, LDQ_L, LDBU, LDWU, STL, STQ, STB, STW |
| Branch | 0x30–0x3F | BR, BSR, BEQ, BNE, BLT, BLE, BGT, BGE, BLBC, BLBS |
| Jump | 0x1A | JMP, JSR, RET, JSR_COROUTINE |

#### Key design decisions

- **Little-endian** memory layout throughout (unique in this series; all prior
  simulators are big-endian)
- **HALT = 0x00000000** — uninitialized memory halts cleanly
- **No condition codes** — compare instructions write 0/1 to GPRs
- **No delay slots** — branches take effect immediately
- **ADDL/SUBL/MULL sign-extend** their 32-bit result to 64 bits
- **Operate literal is 8-bit zero-extended** (not sign-extended)
- **r31 hardwired zero** — reads return 0, writes silently discarded
- **Alignment checking** — quadword (8), longword (4), word (2) operations
  raise `ValueError` on unaligned access
- **LDL_L / LDQ_L** treated as LDL / LDQ (no lock/store-conditional emulation)

#### Test coverage

- 146 tests across four modules: `test_protocol`, `test_instructions`,
  `test_coverage`, `test_programs`
- 88.83% line coverage (target: ≥80%)
- End-to-end programs: sum 1–10, factorial, Fibonacci, dot product, byte copy,
  bubble sort (sorting network), UMULH 128-bit multiply, subroutine call/return
