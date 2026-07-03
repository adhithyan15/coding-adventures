# aarch64-backend

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
