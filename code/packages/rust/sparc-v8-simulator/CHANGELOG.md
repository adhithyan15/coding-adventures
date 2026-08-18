# Changelog — sparc-v8-simulator

## [0.1.0] - 2026-08-17

### Added

- Rust port of
  `code/packages/python/sparc-v8-simulator/src/sparc_v8_simulator/{state,simulator}.py`
  (Layer 07r): full Format 1/2/3 instruction coverage, PSR condition
  codes, `Y` register, overlapping register windows (`SAVE`/
  `RESTORE`), big-endian memory, `ta 0`-as-HALT convention, no
  branch-delay slots.
- Modular architecture mirroring `mips-r2000-simulator`, plus a
  SPARC-specific `registers.rs` module for the windowed register file
  (`virt_to_phys`, cross-checked bit-for-bit against
  `sparc-v8-gatelevel::register_file::virt_to_phys`).
- `SparcV8Simulator` public API mirrors `MipsR2000Simulator`'s shape:
  `new(memory_size)`, public `regs`/`mem`/`psr`/`y`/`pc`/`halted`
  fields, `load_program`, `run`, `run_loaded_with_limit` returning
  `ExecutionResult { halted, steps, pc }`, `step() -> String`.
- Fail-closed halt (instead of the Python original's `ValueError`) for
  `UDIV`/`SDIV` by zero, register-window overflow (`SAVE` past
  `NWINDOWS - 1` nesting), and non-`TA` `Ticc` traps.
- Full register-window machinery ported -- `SAVE`/`RESTORE` are not
  stubbed in this crate (only `sparc-v8-backend`'s v0.1.0 CIR lowering
  scopes itself to globals-only; the simulator underneath is complete).
- 40+ unit tests: every Format 1/2/3 instruction family, encode-decode
  round trip, register-window rotation (including overflow detection
  and outs-alias-ins-of-next-window), big-endian load/store,
  divide-by-zero fail-closed halt, `CALL`/`JMPL`, `Bicc` branches
  (taken/not-taken/backward loop), and the canonical
  `ADD %g0, 42, %o0; ta 0` "load immediate + halt" sequence the
  `sparc-v8-backend` smoke test relies on.

First Rust simulator for this architecture; sixth lane of the
9-architecture expansion following the pattern documented in
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
