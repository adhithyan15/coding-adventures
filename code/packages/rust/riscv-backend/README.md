# riscv-backend

RV32I backend for `jit-core` / `aot-core`.  Phase 7 (the FINAL
lane) of the historical-arch backend migration — see
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).

## What's inside

* `Backend` trait impl (`Riscv32Backend`) plugging RV32I into
  the same registry as `aarch64-backend` and `x86_64-backend`.
* CIR → `Vec<u8>` lowering via `riscv-encoder`.
* Output format: little-endian 32-bit instruction words.
  Per-function bytes can be concatenated directly — `lang-aot`
  flattens them straight into a `.bin`.

## Current scope — executable scalar core

Same scope as `intel8008-backend` v0.1.0 and `armv7-backend`
v0.1.0: just enough to keep the existing lang-aot RV32I e2e smoke
tests passing byte-for-byte.

| CIR family | Status |
|------------|--------|
| `const_*` | ✓ RV32 scalars and full-width `i64`/`u64` register pairs |
| Integer scalar ops | ✓ `add`, `sub`, `and`, `or`, `xor`, `shl`, `shr`, `neg`, `not` |
| Wide integer ops | ✓ `add_i64`, `sub_i64`, `add_u64`, `sub_u64` |
| Comparisons | ✓ signed/unsigned `eq ne lt le gt ge` |
| Control flow | ✓ `label`, `jmp`, `jmp_if_true`, `jmp_if_false` |
| `ret_*`, `ret_void` | ✓ scalar result in `a0`; wide result in `a0:a1` |
| Wide multiply/divide/bitwise/shifts/comparisons, floats, calls, memory, I/O | not yet supported |

`run_binary` executes a produced function binary in `riscv-simulator` and
reports the `a0` low return word, `a1` high return word, halt state, and
instruction count. It installs a one-word `ecall` return trampoline, so normal
function binaries remain usable both as callable code and as inputs to the
simulator runner.

```rust
use jit_core::backend::FunctionContext;
use jit_core::cir::{CIRInstr, CIROperand};
use riscv_backend::{compile, run_binary};

let cir = vec![
    CIRInstr::new("const_i32", Some("answer"), vec![CIROperand::Int(42)], "i32"),
    CIRInstr::new("ret_i32", None::<&str>, vec![CIROperand::Var("answer".into())], "i32"),
];
let context = FunctionContext { name: "main", params: &[], return_type: "i32" };
let binary = compile(&context, &cir).unwrap();
let result = run_binary(&binary, &[]).unwrap();
assert_eq!(result.return_value, 42);
```

For compatibility with existing scalar source smoke tests, a word-sized
`const_i64` / `ret_i64` retains the original canonical bytes. Wider constants
and `add` / `sub` values use a low/high register pair. Wide comparisons remain
limited to word-sized operands until pair-aware ordering sequences land.

## Why this is the FINAL lane

The RV32I backend was the **original** target the A1+ cascade
shipped at the wrong layer (IIR-direct) in May 2026.  It's the
mistake that started the whole pattern Phases 1–6 corrected; the
migration spec docs (`HISTORICAL-ARCH-BACKEND-MIGRATION.md`)
describe RV32I as "last (the original mistake from A1+ that
started this whole pattern)".  Phase 7 closes the loop.
