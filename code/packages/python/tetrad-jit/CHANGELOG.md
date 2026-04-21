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
