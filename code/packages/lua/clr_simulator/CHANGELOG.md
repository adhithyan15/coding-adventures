# Changelog — coding-adventures-clr-simulator

## 0.1.0 — 2026-03-31

### Added

- Initial Lua port of the CLR simulator (Elixir reference: `elixir/clr_simulator`).
- Full educational simulator for a subset of CLR Intermediate Language (IL/CIL/MSIL).
- Opcodes: nop, ldnull, ldc.i4.0-8, ldc.i4.s, ldc.i4, ldloc.0-3, stloc.0-3, ldloc.s, stloc.s, ret, br.s, brfalse.s, brtrue.s, add, sub, mul, div, ceq (0xFE01), cgt (0xFE02), clt (0xFE04).
- Assembly helpers: `encode_ldc_i4`, `encode_stloc`, `encode_ldloc`, `assemble`.
- Step tracing: every executed instruction produces a trace with pc, opcode, stack_before, stack_after, locals snapshot, and plain-English description.
- `run()` executes until halt or max_steps limit.
- 95%+ test coverage with busted.
