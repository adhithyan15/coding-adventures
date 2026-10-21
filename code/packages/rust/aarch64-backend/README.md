# aarch64-backend

Register the two-argument str_index checked runtime builtin in the generic ABI table.

Register the three-argument str_slice runtime builtin for generic ABI marshalling.

ARM64 (AArch64) native-code backend for `jit-core` and `aot-core`.  Lowers
CIR to ARM64 machine code via `aarch64-encoder`.

## Stack position

```
IIRModule (interpreter-ir)
   │
   ▼ aot-core::specialise / jit-core::specialise
CIR (jit-core::cir)
   │
   ▼ aarch64-backend::AArch64Backend  (this crate)
Vec<u8> ARM64 machine code
   │
   ▼ code-packager::macho64 / elf64 / pe
runnable binary
```

## Trait wiring

Implements `jit_core::backend::Backend` via the new `compile_function`
method (richer than `compile`: receives a `FunctionContext` with name,
parameter list, and return type — needed for AAPCS64 prologue layout).

## Status

V1: stack-spill register allocation, integer arithmetic + comparisons +
control flow + returns.  Enough to compile typed Twig functions like
`fib`, `fact`, `sum`.  See CHANGELOG for the full opcode list.

McCarthy 1960 Lisp (LANG77) compiles end-to-end through this backend via the
`__twig_lispy_*` runtime calls in `V1_BUILTINS` — cons/car/cdr, the ATOM/EQ
predicates, COND truthiness, and (W14b) `LAMBDA`: cross-function `call`s plus
`lispy_to_exit_code` (the polymorphic program-exit coercion) make native lambda run.

## TWIG-GC integration (native-aot-substrate PR-1)

The `alloc` IIR op now calls `__twig_gc_alloc(size)` (TWIG-GC) instead of the
leaking `__twig_alloc_bytes`.  Size is read from `srcs[0]`; defaults to 16
(LispyPair) if absent.  The `safepoint` IIR op lowers to
`BL __twig_gc_safepoint` — a no-arg call that triggers GC when the live-byte
threshold is exceeded.  Two new V1_BUILTINS — `gc_alloc` and `gc_safepoint` —
let frontends emit `call_builtin "gc_alloc"` / `"gc_safepoint"` directly.

Later passes will add: real register allocation, float operations,
runtime-call lowering, deopt support for JIT.

FLOW-MATIC EOF support (VM-039a): `input_more()` uses a non-consuming stdio
peek alongside `input_i64`. The native/LLVM input stream proof includes empty
input and repeated EOF reads; this does not change existing integer parsing.

## LANG83 operation support

`aarch64_backend::op_support::op_support(name)` (and `OP_SUPPORT`) is this backend's answer,
`Lowered` or `Refused(reason)`, for every operation in the
[LANG83](../../../specs/LANG83-iir-standard-operation-registry.md) standard registry
(`interpreter_ir::op_registry::STANDARD_OPS`). "Lowered" means the native AOT pipeline that ends in this backend (twig-aot's IIR rewriting, then `jit_core::specialise`, then this crate) compiles the op. This crate dispatches CIR ops by name prefix, which is why LANG83 requires new op names to be type-first. `lang-aot` refuses a
module that uses a refused op at compile time, with a typed error. Since LANG83 PR1b this backend also lowers `write_i64`
(`__twig_write_i64`), `i64_div_floor`/`i64_mod_floor` (explicit zero and
`MIN / -1` checks before `SDIV`, which would otherwise silently return 0 or
`MIN`) and `trap` (`bl __twig_trap` with the stable kind id, then `udf`).
LANG83 PR4 adds the overflow-checked `i64_add_ovf`/`i64_sub_ovf`/`i64_neg_ovf`
(`adds`/`subs`/`negs`, then `b.vc` over a `Trap.Overflow` trap call) and
`i64_mul_ovf` (`mul` + `smulh`, `cmp hi, lo, asr #63`, `b.eq` over the trap).
