## Unreleased — 2026-10-26 — Trap.ConvRange on every backend

### Changed

- A `real_to_int_trunc` / `real_to_int_floor` that cannot fit (NaN, ±∞, a
  value outside `[-2⁶³, 2⁶³)`) now stops with LANG83's typed
  `Trap.ConvRange` on every backend, after flushing the output written
  before it. `op_conformance` gains ten `StdoutThenTrap("1",
  "Trap.ConvRange")` cases (five values × two ops) and the gate
  `failed_narrowing_traps_conv_range_on_every_executable_column`, which runs
  them on WASM, VM, JIT, LLVM, native and JVM and compiles them for CLR and
  (finite rows) BEAM. A `-2⁶³` case pins the inclusive lower bound.
- The two ALGOL out-of-range `entier` rows in `lang_matrix` expect
  `StdoutThenTrap("", "Trap.ConvRange")` instead of the untyped `Trap`, so
  every column must report the kind.
