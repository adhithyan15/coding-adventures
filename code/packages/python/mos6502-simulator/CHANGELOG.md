# Changelog — mos6502-simulator

All notable changes to this project will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-05-04

### Added

**Package (Layer 07j — MOS 6502 NMOS behavioral simulator)**

- `MOS6502State` frozen dataclass — complete snapshot of all registers (A, X,
  Y, S, PC), all seven status flags (N, V, B, D, I, Z, C), halt flag, and the
  full 64 KiB memory tuple; includes `p_byte()` helper that packs flags into
  the 6502 P register byte (bit 5 always 1).

- `MOS6502Simulator` implementing the SIM00 `Simulator[MOS6502State]` protocol
  from `coding-adventures-simulator-protocol`:
  - `reset()` — clears registers; S initialised to `$FD`; I flag set; 64 KiB
    memory zeroed.
  - `load(program, origin=0x0000)` — validates origin in `[0, 0xFFF0]`, copies
    bytes into memory, sets PC.
  - `step()` → `StepTrace` — fetches and executes one instruction; raises
    `RuntimeError` if already halted.
  - `execute(program, origin, max_steps)` → `ExecutionResult[MOS6502State]` —
    runs until BRK / `max_steps`; preserves port arrays across the internal
    `reset()` call.
  - `get_state()` → `MOS6502State` — returns a frozen snapshot.
  - `set_input_port(port, value)` / `get_output_port(port)` — 240 memory-mapped
    I/O ports at `$FF00`–`$FFEF`.

- **All 13 addressing modes** resolved by `_resolve_address()`:
  Immediate, Zero Page, Zero Page,X, Zero Page,Y, Absolute, Absolute,X,
  Absolute,Y, (Indirect,X), (Indirect),Y, Implied, Accumulator, Relative,
  Indirect.  The NMOS page-wrap bug in Indirect mode (`JMP ($xxFF)` reads
  high byte from `$xx00` instead of `$(xx+1)00`) is faithfully reproduced.

- **151 opcode entries** covering all NMOS 6502 instructions:
  - *Load/Store*: LDA, LDX, LDY, STA, STX, STY (8 modes each where applicable)
  - *Arithmetic*: ADC, SBC (binary and NMOS BCD), INC, DEC, INX, INY, DEX, DEY
  - *Logical*: AND, ORA, EOR, BIT
  - *Shift/Rotate*: ASL, LSR, ROL, ROR (accumulator and memory)
  - *Branch*: BCC, BCS, BEQ, BMI, BNE, BPL, BVC, BVS (signed 8-bit relative)
  - *Jump/Call*: JMP (absolute, indirect), JSR, RTS, RTI
  - *Transfer*: TAX, TAY, TXA, TYA, TSX, TXS
  - *Stack*: PHA, PLA, PHP, PLP
  - *Flag*: CLC, SEC, CLD, SED, CLI, SEI, CLV
  - *Compare*: CMP, CPX, CPY
  - *System*: BRK (halts simulation; pushes PC+1 and P with B=1; sets I=1)

- **NMOS BCD mode** — when D=1, ADC/SBC correct the binary result using
  nibble-by-nibble BCD adjustment; N/V/Z are set from the binary intermediate
  (accurate to NMOS behaviour; the 65C02 differs here).

- `flags.py` — pure helper functions:
  `compute_nz`, `compute_overflow_add`, `compute_overflow_sub`,
  `pack_p`, `unpack_p`, `bcd_add`, `bcd_sub`.

- **Test suite — 162 tests, 94.46% coverage** (well above the 80% threshold):
  - `test_flags.py` — flag helper unit tests (N/Z, overflow, pack/unpack, BCD)
  - `test_protocol.py` — SIM00 interface compliance (reset, load, step,
    execute, ports, max_steps guard, get_state)
  - `test_load_store.py` — LDA (all 8 addressing modes), LDX/LDY, STA/STX/STY
  - `test_arithmetic.py` — ADC (binary + BCD), SBC (binary + BCD), INC/DEC
    memory, INX/INY/DEX/DEY (wrap behaviour)
  - `test_logical.py` — AND/ORA/EOR, BIT (N/V from memory bits 7–6), ASL/LSR/
    ROL/ROR accumulator + memory
  - `test_branch.py` — all 8 branch conditions taken/not-taken, backward loop,
    JMP absolute, JMP indirect page-wrap bug, JSR/RTS round-trip, RTI
    (restores flags + PC without the +1 that RTS adds)
  - `test_transfer.py` — TAX/TAY/TXA/TYA/TSX/TXS (TXS does not set flags),
    CMP/CPX/CPY, all 7 flag instructions (CLI/SEI checked before BRK fires)
  - `test_stack.py` — PHA/PLA round-trip, PLA sets N/Z, LIFO order, PHP/PLP
    round-trip including B and bit-5 in pushed byte
  - `test_programs.py` — end-to-end programs: sum 1..N, multiply, Fibonacci
    (7th term, store sequence to memory), double and nested subroutine calls

### Notes

- Zero-page data in test programs uses addresses `$40`+ to avoid the NMOS
  6502's flat memory model causing self-modification when code is loaded at
  `$0000` and ZP data addresses overlap with code bytes (e.g., `$10`–`$1F`).
- BRK pushes to the stack and sets I=1; tests that verify S or flag_i after an
  instruction check state via `step()` rather than relying on the final state
  after `execute()`.
