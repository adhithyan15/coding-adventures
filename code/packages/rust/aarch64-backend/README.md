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

Later passes will add: real register allocation, float operations,
runtime-call lowering, deopt support for JIT.
