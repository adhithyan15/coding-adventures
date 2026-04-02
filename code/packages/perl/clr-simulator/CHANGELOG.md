# Changelog — CodingAdventures::ClrSimulator

## 0.01 — 2026-03-31

### Added

- Initial Perl port of the CLR simulator (Elixir reference: `elixir/clr_simulator`).
- All CLR opcodes: nop, ldnull, ldc.i4 variants (0-8, .s, full 32-bit), ldloc/stloc (0-3 and .s forms), ret, br.s, brfalse.s, brtrue.s, add, sub, mul, div, ceq/cgt/clt (0xFE prefix).
- Assembly helpers: encode_ldc_i4, encode_stloc, encode_ldloc, assemble.
- Step tracing with pc, opcode, stack_before/after, locals, description.
- run() with max_steps support.
- 95%+ test coverage with Test2::V0.
