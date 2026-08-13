# riscv-backend

RV32I backend for `jit-core` / `aot-core`, using the RV32M multiply subset
for full-width products. Phase 7 (the FINAL
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
| Integer scalar ops | ✓ `add`, `sub`, `mul`, `div`, `mod`, `and`, `or`, `xor`, `shl`, `shr`, `neg`, `not` |
| Wide integer ops | ✓ `add_i64`, `sub_i64`, `mul_i64`, `div_i64`, `mod_i64`, plus unsigned forms |
| Wide bitwise ops | ✓ `and_i64`, `or_i64`, `xor_i64`, `not_i64` and unsigned forms |
| Wide shifts | ✓ left, logical-right, and arithmetic-right shifts for counts `0..63` |
| Wide multiplication | ✓ signed and unsigned full-width products via RV32M `mul` / `mulhu` |
| Comparisons | ✓ signed/unsigned `eq ne lt le gt ge`, including `i64`/`u64` pairs |
| Control flow | ✓ `label`, `jmp`, `jmp_if_true`, `jmp_if_false` |
| `ret_*`, `ret_void` | ✓ scalar result in `a0`; wide result in `a0:a1` |
| Calls, memory, I/O | not yet supported |
| Floating point (`f32`/`f64`) | ✗ **refused by design** — see below |

### Floating point is refused, not "unimplemented"

RV32I is the RISC-V *base integer* ISA: 32 integer registers, no `f0`..`f31`
bank, no `fadd.d`.  Single and double precision live in the optional `F` and
`D` standard extensions (RV32F / RV32D).  An `f64` here is therefore not an op
nobody has written yet — it is a value the target cannot hold, and the two
cases have opposite fixes.  So a float gets its own error,
`BackendError::UnsupportedFloat { site, ty }`, which names the CIR op (or the
parameter) that carried it and says plainly that RV32I has no floating-point
registers.  Never approximate: truncating a double to an integer to produce
*some* bytes would be a silent wrong answer.

The real-world case is Dartmouth BASIC, whose one numeric type is REAL — after
the BA7 floating-point conversion even `PRINT 42` is `const_f64 42.0`, so no
BASIC program reaches RV32I bytes today.  The routes forward are retargeting
(LLVM/JVM/CLR/wasm all carry doubles) or a future soft-float pass that
decomposes a double into integer sequences before this backend sees it.

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
and `add` / `sub` values use a low/high register pair. Pair comparisons use
signed or unsigned high-word ordering, then unsigned low-word ordering when
the high words are equal. Pair bitwise operations apply independently to the
low and high words. Pair shifts distinguish zero, sub-word, cross-word, and
out-of-range counts before using the RV32 shift instructions. Unsigned pair
division and modulo use a 64-step restoring loop, including RV32M-compatible
zero-divisor results. Signed pair operations normalize magnitudes around that
loop and preserve `i64::MIN / -1` two's-complement behavior.

## Why this is the FINAL lane

The RV32I backend was the **original** target the A1+ cascade
shipped at the wrong layer (IIR-direct) in May 2026.  It's the
mistake that started the whole pattern Phases 1–6 corrected; the
migration spec docs (`HISTORICAL-ARCH-BACKEND-MIGRATION.md`)
describe RV32I as "last (the original mistake from A1+ that
started this whole pattern)".  Phase 7 closes the loop.
