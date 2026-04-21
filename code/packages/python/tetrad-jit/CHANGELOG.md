# Changelog — coding-adventures-tetrad-jit

## [0.1.0] — 2026-04-21

### Added

- `ir.py` — `IRInstr` SSA dataclass; `ARITHMETIC_OPS`, `CMP_OPS`, `BINARY_OPS`,
  `SIDE_EFFECT_OPS` frozensets; `evaluate_op()` constant evaluator.
- `translate.py` — Tetrad bytecode → JIT IR translator; handles all Tetrad
  opcodes; resolves jump offsets to label names in a pre-scan pass.
- `passes.py` — Two optimization passes: constant folding (forward values
  propagation with `evaluate_op`) and dead code elimination (backward liveness).
- `codegen_4004.py` — Intel 4004 code generator; register pair allocation
  (P0–P5 for virtual vars, P6 for RAM address, P7 for temps); 8-bit arithmetic
  via nibble-pair sequences (add, sub, cmp_eq/ne/lt/le/gt/ge); JZ/JNZ via
  dual nibble checks; two-pass assembler that resolves labels to ROM addresses;
  `run_on_4004()` helper for executing compiled binaries on `Intel4004Simulator`.
- `cache.py` — `JITCacheEntry` dataclass; `JITCache` dict-backed store with
  `get / put / invalidate / stats` API.
- `__init__.py` — `TetradJIT` public class with `compile`, `is_compiled`,
  `execute`, `execute_with_jit`, `cache_stats`, `dump_ir`.

### Fixed

- **BUILD**: removed `set -euo pipefail` (bash-only; BUILD files run under `sh`).
- **`execute()`**: interpreter fallback now builds a synthetic caller CodeObject
  (LDA_IMM + STA_REG + CALL + RET) instead of calling the non-existent
  `TetradVM.call_function()` method.
- **`execute_with_jit()`**: runs `main()` by building a synthetic CodeObject
  with `main.instructions` and the full `code.functions` list so CALL
  instructions can resolve function indices correctly.
- **SUB semantics**: `Intel4004Simulator.SUB` computes `A = A + ~Rn + (1−CY)`,
  not `+CY`.  Fixed `_emit_sub` to use `CLC` + `CMC` between nibbles.
- **CMP semantics**: all comparison emitters updated from `STC` to `CLC`
  (equality check: CLC gives A=0 for equal nibbles; STC gave A=15).
- **Liveness-based register recycling**: `_pair_of()` now pre-scans IR to find
  each variable's last use and recycles dead pairs before allocating fresh ones.
  Functions with ≤6 simultaneously-live variables now compile even when the
  total SSA variable count exceeds 6 (e.g. `if`-branching functions that
  previously deopted due to the 6-pair limit now compile cleanly).

### Design decisions

- **Target: Intel 4004, not x86-64.** The original TET05 draft specified
  x86-64 + ctypes/mmap.  This was replaced because Tetrad's architecture is
  modelled on the 4004, the `intel4004-simulator` already exists in this repo,
  and the educational goal is clearer when the JIT targets the same hardware
  the VM was inspired by.
- **Deopt for unsupported ops.** Mul, div, bitwise, I/O, and function calls
  are not yet supported.  `compile()` returns `False`; the interpreter handles
  those functions.
- **u8 via register pairs.** The 4004 is 4-bit; Tetrad values are 8-bit.
  Each u8 is stored as hi nibble in R(2p) and lo nibble in R(2p+1).  `FIM Pp,
  d8` loads both nibbles in one 2-byte instruction.
