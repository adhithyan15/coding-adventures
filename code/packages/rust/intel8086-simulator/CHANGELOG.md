# Changelog — intel8086-simulator

## [0.1.0] - 2026-08-17

### Added

- Rust port of `code/packages/python/intel-8086-simulator` (Layer 07m): a
  curated core of the Intel 8086 (1978) instruction set — register-
  immediate and register-to-register data transfer, accumulator- and
  register-to-register ALU ops (`ADD`/`SUB`/`AND`/`OR`/`XOR`/`CMP`),
  `INC`/`DEC reg16`, and the genuine `HLT` halt instruction — plus the
  segmented 20-bit physical-addressing model (`physical = CS<<4 + IP`)
  that every instruction fetch goes through, ported faithfully rather
  than flattened.
- Modular architecture: `opcodes` (curated opcode table + register-index
  constants + `HLT_OPCODE`), `flags` (CF/PF/AF/ZF/SF/OF computation,
  ported from `flags.py`), `decode` (fetch + operand decode, register-
  only ModRM), `execute` (instruction dispatch over
  `&mut Intel8086Simulator`), `simulator` (top-level fetch-decode-execute
  loop + `phys_addr`), `encoding` (`encode_*` helpers for the subset of
  mnemonics exercised by tests / `intel8086-encoder`).
- `Intel8086Simulator` public API mirrors `Mos6502Simulator`'s shape:
  `new(memory_size)`, public register/flag/`mem`/`halted` fields,
  `load_program`/`load_program_at`, `run`, `run_loaded_with_limit`
  returning `ExecutionResult { halted, steps, ip }`, `step() -> String`.
- `HLT` (opcode `0xF4`) is the halt sentinel — a genuine single-byte
  hardware halt instruction ported directly from the Python original's
  `if op == 0xF4: self._halted = True`, unlike ARM1's invented pseudo-
  halt (`SWI`) or MOS 6502's repurposed `BRK`.
- Fail-closed halt on an unsupported/illegal opcode byte (instead of the
  Python original's "treat as HLT" fallback) — no exception channel
  exists through `step() -> String`, so the simulator stops rather than
  silently misinterpreting bytes, the same pattern `mos6502-simulator`
  uses for its own illegal-opcode case.
- Register-only ModRM decoding: `mod=11` (register-to-register) forms
  resolve; `mod != 11` (memory operand) is a decode `Err`, not a silent
  misdecode — memory effective-address computation (`[BX+SI]` and
  friends) is explicitly deferred to a future increment.
- 61 unit tests across all six modules: opcode-table sanity (every
  supported opcode range, an unsupported-opcode rejection), flag
  computation matched against the Python reference's documented
  doctstring examples (carry, auxiliary carry, overflow, sign/zero/
  parity), decode of every supported instruction shape (including the
  register-only ModRM rejection and a nonzero-`CS`-segment fetch proof),
  execute-level behaviour (`MOV` in all its supported forms, ALU ops
  including `CMP`'s no-writeback semantics, `INC`/`DEC` CF preservation),
  the segmented physical-address formula itself (zero case, a classic
  boot-sector `CS=0x07C0` case, and 20-bit wraparound), and simulator-
  level integration tests — including the canonical `MOV AX,42; HLT`
  "load immediate into the accumulator + halt-convention" sequence the
  `intel8086-backend` smoke test relies on.

### Deferred (out of scope for v0.1.0)

Memory-operand addressing (`[BX+SI]`, `[BP+DI+disp8]`, `[disp16]`, and
every `mod != 11` ModRM form); segment-override prefixes; `LOCK`;
`REP`/`REPNE` string-op prefixes and the string operations themselves
(`MOVS`/`CMPS`/`STOS`/`LODS`/`SCAS`); stack operations (`PUSH`/`POP`/
`PUSHF`/`POPF`/`CALL`/`RET`); control flow (`JMP`, conditional jumps,
`LOOP`, interrupts); `MUL`/`IMUL`/`DIV`/`IDIV`; the shift/rotate group;
BCD adjust instructions (`DAA`/`DAS`/`AAA`/`AAS`/`AAM`/`AAD`); `XCHG`,
`LEA`, `LDS`/`LES`, `LAHF`/`SAHF`, `CBW`/`CWD`, `XLAT`; `MOV sreg,r/m` /
`MOV r/m,sreg`; I/O port instructions and memory-mapped port emulation;
`TF`/`IF` flag semantics beyond storage (no interrupt or single-step
machinery).

Ninth and final lane of the 9-architecture expansion following the
pattern documented in
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
