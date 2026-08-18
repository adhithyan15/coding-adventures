# Changelog — m68k-simulator

## [0.1.0] - 2026-08-17

### Added

- Rust port of `code/packages/python/motorola-68000-simulator` (Layer
  07n): a substantial subset of the bit-field-decoded Motorola 68000
  instruction set — `MOVE`/`MOVEA`/`MOVEQ`, `ADD`/`SUB`/`AND`/`OR`/`EOR`/
  `CMP` (register and memory forms), `ADDQ`/`SUBQ`, `Scc`/`DBcc`,
  `BRA`/`BSR`/all 14 `Bcc`, register-form shift/rotate (`ASL`/`ASR`/
  `LSL`/`LSR`/`ROXL`/`ROXR`/`ROL`/`ROR`), `CLR`/`NEG`/`NOT`/`TST`,
  `SWAP`/`EXT.W`/`EXT.L`, `EXG`, `LEA`/`JSR`/`JMP`, `NOP`/`RTS`/`RTR`/
  `STOP`/`TRAP`/`LINK`/`UNLK`.
- 8 of the 11 non-register-direct effective-address variants: register
  direct (`Dn`/`An`), indirect (`(An)`), postincrement (`(An)+`),
  predecrement (`-(An)`), 16-bit displacement (`d16(An)`), absolute
  short/long (`(abs).W`/`(abs).L`), and immediate (`#imm`). The 3
  indexed/PC-relative modes (`d8(An,Xn.sz)`, `d16(PC)`, `d8(PC,Xn.sz)`)
  are deferred — see `decode.rs`'s module doc and the README's
  addressing-modes table.
- Modular architecture: `opcodes` (shared size-code tables,
  condition-code predicates, the HALT sentinel), `decode` (effective-
  address classification/resolution + PC-relative fetch helpers),
  `flags` (N/Z/V/C/X computation, direct port of `flags.py`), `execute`
  (one function per opword "line" — the 68000 has no flat opcode table
  the way MOS 6502/8080-family CPUs do, so there's no single lookup
  table to port; each `_exec_line*` Python method becomes its own Rust
  function), `simulator` (top-level fetch-decode-execute loop),
  `encoding` (`encode_*` helpers for the subset of mnemonics exercised
  by tests / `m68k-encoder`).
- `M68kSimulator` public API mirrors every other Rust ISA simulator in
  this repo: `new(memory_size)`, public `d`/`a` register-array fields
  (`[u32; 8]` each — 16 uniform 32-bit GPRs, unlike the 6502's 3 small
  irregular named registers), `pc`/`sr`/`halted`/`mem` fields,
  `load_program`/`load_program_at`, `run`, `run_loaded_with_limit`
  returning `ExecutionResult { halted, steps, pc }`, `step() -> String`.
  CCR flag accessor methods (`flag_n`/`flag_z`/`flag_v`/`flag_c`/
  `flag_x`) mirror the Python original's `M68KState` properties.
- **Halt convention: `TRAP #15`, not `STOP #imm`.** The pre-existing
  Python simulator's `state.py` documents both as valid halting
  conditions ("halted: True after STOP or TRAP #15 executes"), but its
  own test suite's `_stop()` helper — used 100+ times across
  `test_instructions.py`/`test_programs.py` — is `TRAP #15`; `STOP
  #imm` appears exactly once, in a module-level doctest. `TRAP #15` is
  therefore the dominant, already-established idiom this port mirrors,
  per this repo's rule of reusing what a pre-existing reference already
  does rather than inventing a fresh convention. `STOP #imm` is still
  ported faithfully for programs that use it directly.
- Big-endian memory access throughout (`mem_read`/`mem_write`,
  `fetch_word`/`fetch_long`) — the 68000's native byte order, unlike
  every other Rust ISA simulator in this repo (MIPS R2000/ARM1/RV32I are
  little-endian; MOS 6502 has no word endianness at all).
- Fail-closed halt on any decode/execute failure (illegal opword, a
  deferred addressing mode or instruction family, a misaligned
  word/long access) — no exception channel exists through
  `step() -> String`, so the simulator stops rather than silently
  corrupting state or panicking, the same pattern `mos6502-simulator`
  uses for illegal opcodes. The Python original raises a Python
  exception for the same conditions instead.
- 55+ unit tests across all six modules: size-code/mask/condition-code
  tables, EA classification and resolution (including the
  postincrement/predecrement stack-alignment rule and misaligned-access
  rejection), flag helpers (carry/overflow/N/Z for both ADD-family and
  SUB-family arithmetic, including the arbitrary-precision-`raw`
  reasoning needed to reproduce the Python original's negative-`raw`
  masking exactly), and simulator-level integration tests — including
  the canonical `MOVE.L #42, D0; TRAP #15` "load immediate into D0 +
  halt-convention" sequence the `m68k-backend` smoke test relies on.

Eighth lane of the 9-architecture expansion following the pattern
documented in
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
