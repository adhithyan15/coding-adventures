# Changelog — z80-simulator

## [Unreleased]

### Changed

- Completed all ED operations represented by the Python reference:
  16-bit arithmetic/load families, special-register transfers, nibble
  rotates, interrupt control, and repeating/non-repeating transfer,
  compare, input, and output blocks.
- Completed DD/FD displacement, arithmetic, stack, branch, and direct
  forms plus every DDCB/FDCB rotate/shift/bit/set/reset form.
- Replaced the variable-memory/string lifecycle with an architectural
  64 KiB model, complete typed state/snapshots/traces, atomic checked
  loading and stepping, transactional bounded runs, checked ports, and
  maskable/NMI interrupt entry.
- Added a deterministic full-state differential against the Python
  simulator for 1,160 defined prefixed and unprefixed opcode vectors.

## [0.1.0] - 2026-08-17

### Added

- Rust port of `code/packages/python/z80-simulator`: the base
  8080-compatible instruction set (byte-identical encoding to
  `intel8080-simulator`) plus the Z80's own additions — alternate
  register bank (`EX AF,AF'`/`EXX`), relative jumps (`DJNZ`/`JR` + 4
  conditional forms), `CB`-prefixed bit manipulation (`BIT`/`RES`/`SET`)
  and extended rotate/shift (`RLC`/`RRC`/`RL`/`RR`/`SLA`/`SRA`/`SLL`/
  `SRL`), and IX/IY basics (`LD IX/IY,nn`, `INC IX/IY`).
- Modular architecture mirroring `mips-r2000-simulator` /
  `intel8080-simulator`: opcodes, encoding, decode, execute, simulator.
- `Z80Simulator` public API mirrors `Intel8080Simulator`'s shape:
  `new(memory_size)`, public `regs`/`flags`/`mem`/`pc`/`halted`/`iff1`/
  `iff2`/`im` fields, `load_program`, `run`, `run_loaded_with_limit`
  returning `ExecutionResult { halted, steps, pc }`, `step() -> String`.
- Named-register `Registers` struct (main bank A/B/C/D/E/H/L + alternate
  bank A'/F'/B'/C'/D'/E'/H'/L' stored as raw bytes + IX/IY/SP/I/R) instead
  of an indexed `RegisterFile`, mirroring the 8080 port's approach.
- `Flags` struct with the six named Z80 flags (S, Z, H, P/V, N, C) — an
  extra `N` flag versus the 8080 port, and a dual-purpose `P/V` (parity
  after logical ops, signed overflow after arithmetic ops).
- Variable-length decode (`decode::decode(first_byte, fetch)`) supporting
  1-4 byte instructions, with a 4-way prefix dispatch (`CB`/`ED`/`DD`/
  `FD`) on top of the 8080-style unprefixed decode.
- Initial scope intentionally left ED and most IX/IY displacement forms
  for a later completion audit; those are now covered under Unreleased.
- 35+ unit tests: ALU ops (including the Z80's dual-purpose P/V and the
  new N flag), INC/DEC wrap + signed-overflow detection, load/store via
  direct address and `(HL)`, unconditional/conditional `JP`, `JR`
  (forward + backward loop), `DJNZ` loop, `CALL`/`RET`, `RST`, `PUSH`/
  `POP` (including `PUSH AF`/`POP AF` round-tripping flags), rotates,
  `DAA` (BCD correction), I/O ports, `EI`/`DI`, `ED`-prefix fail-closed
  halt, `EX AF,AF'`/`EXX` bank swaps, `CB`-prefixed `RLC`/`BIT`/`SET`/
  `RES`, `DD`/`FD` IX/IY `LD`/`INC`, a full encode-decode round trip
  across every ported mnemonic, and the canonical `LD A,42; HALT`
  "load immediate into accumulator + halt" sequence the `z80-backend`
  smoke test relies on (byte-identical to `intel8080-simulator`'s
  `MVI A,42; HLT`).

Seventh lane of the 9-architecture expansion following the pattern
documented in
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
