# Changelog — sparc-v8-simulator

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-05-05

### Added

#### Architecture
- **SPARC V8 behavioral simulator** implementing Sun Microsystems' 1987 RISC
  architecture with register windows
- **56 physical registers**: 8 globals (`%g0–%g7`) and 3 × 16 windowed
  registers (out/local/in sets per window); `%g0` is always zero
- **3 register windows** with `virt_to_phys()` mapping: virtual `%r0–%r31`
  to physical indices based on Current Window Pointer (CWP)
- **`SAVE` / `RESTORE`** rotate the CWP (mod 3); callee's `%i` registers are
  physically the caller's `%o` registers — no register copying
- **Register-window overflow detection**: raising `ValueError` when a fourth
  `SAVE` would corrupt an active window
- **PSR condition codes**: N (negative), Z (zero), V (overflow), C (carry)
  updated by `cc`-suffix instructions
- **Y register** holding the high 32 bits of multiply results; readable via
  `RDY`, writeable via `WRY`
- **64 KiB flat memory** (`bytearray`); big-endian byte order for all multi-
  byte accesses
- **Alignment enforcement**: word (`LD`/`ST`) requires 4-byte alignment;
  halfword (`LDUH`/`LDSH`/`STH`) requires 2-byte alignment; byte ops always
  OK; `ValueError` on violation

#### Instruction set
- `SETHI`, `NOP` (Format 2)
- `ADD`, `ADDcc`, `ADDX`, `ADDXcc` (carry-in from PSR.C)
- `SUB`, `SUBcc`, `SUBX`, `SUBXcc`
- `AND`, `ANDcc`, `ANDN`, `ANDNcc`
- `OR`, `ORcc`, `ORN`, `ORNcc`
- `XOR`, `XORcc`, `XNOR`, `XNORcc`
- `SLL`, `SRL`, `SRA` (logical and arithmetic right shift)
- `UMUL`, `UMULcc`, `SMUL`, `SMULcc` (64-bit result; high 32 bits → Y)
- `MULScc` (multiply step for iterative multiplier)
- `UDIV`, `UDIVcc`, `SDIV`, `SDIVcc` (Y:rs1 / src; div-by-zero raises)
- `RDY`, `WRY`
- `CALL` (Format 1; writes return address to `%o7`)
- `JMPL` (jump-and-link; writes return address to `rd`)
- `SAVE`, `RESTORE` (window rotation)
- `Ticc` (only `ta 0` / HALT is handled; all other trap conditions raise)
- `LD`, `LDUB`, `LDUH`, `LDSB`, `LDSH` (load word/byte/halfword)
- `ST`, `STB`, `STH` (store word/byte/halfword)
- `Bicc`: BA, BN, BE, BNE, BG, BLE, BGE, BL, BGU, BLEU, BCC, BCS,
  BPOS, BNEG, BVC, BVS

#### SIM00 Protocol
- `SPARCSimulator` implements `Simulator[SPARCState]`:
  - `reset()` — zeros all state; PC=0, nPC=4, CWP=0
  - `load(data)` — resets then copies up to 65 536 bytes to memory at 0x0000
  - `step()` — executes one instruction; returns `StepTrace`; no-op when halted
  - `execute(data, max_steps)` — load + run loop; returns `ExecutionResult`
  - `get_state()` — returns frozen `SPARCState` snapshot
- `SPARCState` frozen dataclass: `pc`, `npc`, `regs` (56-tuple), `cwp`,
  `psr_{n,z,v,c}`, `y`, `memory` (65536-tuple), `halted`
- Convenience properties on `SPARCState`: `.g0–.g7`, `.o0–.o7`, `.l0–.l7`,
  `.i0–.i7`, `.sp` (= `i6`), `.fp` (= `i6` alias), `.o7`

#### Tests — 4 modules, comprehensive coverage
- `test_protocol.py` — SIM00 compliance (reset, load, execute, step, get_state)
- `test_instructions.py` — Per-instruction correctness tests
- `test_programs.py` — End-to-end programs: sum 1–10, SMUL, loop, factorial,
  word/byte copy, Fibonacci, bubble sort, subroutine call, SAVE/RESTORE
- `test_coverage.py` — Edge cases: %g0 immutable, CC corner cases, alignment
  faults, div-by-zero, unknown opcodes, window overflow, big-endian layout,
  ADDX carry propagation

#### Package metadata
- `pyproject.toml` with `src` layout, `py.typed` marker, ruff config
- Depends on `coding-adventures-simulator-protocol`

### Simplifications vs. full SPARC V8

- No branch delay slots (branches take effect immediately)
- Only 3 register windows (architectural minimum; standard was 8–32)
- No FPU or coprocessor instructions
- No privileged / supervisor-mode instructions
- No interrupts or trap vectors beyond `ta 0`
- 64 KiB flat address space (no MMU)
