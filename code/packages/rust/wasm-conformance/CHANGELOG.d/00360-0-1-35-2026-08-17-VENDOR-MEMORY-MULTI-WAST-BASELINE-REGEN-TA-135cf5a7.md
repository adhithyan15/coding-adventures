## 0.1.35 — 2026-08-17 — vendor memory-multi.wast; baseline regen (task #92/#112)

### Changed

- Baseline regen: vendored `memory-multi.wast`, previously blocked on
  real multi-memory memarg support (flags-bit `0x40` + memidx across all
  23 memarg opcodes, plus `memory.init`/`memory.fill`'s own memidx --
  see `wasm-execution`/`wasm-wast-parser`/`wasm-validator`'s own
  CHANGELOG entries and `code/specs/W18-wasm-multi-memory-memarg.md`).
  100% pass on every directive kind: 2/2 modules, 4/4 assert_return.

