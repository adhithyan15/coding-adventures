---
category: Native extensions & FFI
---

# Module-local helpers in generated code must not share a name with, or have interposable linkage against, symbols of a runtime linked into the same program

**What went wrong.** LANG83 PR7b added two native-runtime helpers to
`twig_runtime.c` named `__iir_f64_digits(int64_t)` / `__iir_f64_exp10(int64_t)`.
iir-to-llvm already emitted a module-local 3-argument
`define linkonce_odr i32 @__iir_f64_digits(i64, ptr, ptr)`, the PR6a digit
core behind `@__iir_write_f64`. A `linkonce_odr` name is a global, so it can
be interposed: when an LLVM program also calls a `@__twig_*` helper, the
harness links `twig_runtime.c`, and the linker binds every call to the strong
C definition. The 3-argument calls then ran the 1-argument function, which
returned a bogus digit count and left the 32-byte digit buffer uninitialised.
The security review reproduced `f64_digits(1.5) = 0`, `write_f64_py(0.1)`
printing `0.0` and a SIGSEGV on `123.456`, at `-O0` and `-O2`. Every test
passed, because no case combined a runtime-linking op with float output on
the LLVM column. Each piece was tested alone; the hazard only exists in the
link.

**Fix.** The runtime helpers became `__twig_f64_digits` / `__twig_f64_exp10`
(the runtime's own namespace). Every LLVM float helper became
`define internal`, with `private` constants, so nothing linked can replace
it. A conformance case now runs a string op (which links the runtime) with
the writers and the digit ops on every column. Reintroducing the old name and
linkage makes that case print garbage and trap. A test pins the linkage and
the naming rule.

**Do differently.** Give each producer of symbols its own prefix. A runtime
uses `__twig_*` / `__dyn_*`; compiler-emitted module helpers use `__iir_*` with
`internal` (or `private`) linkage. Use `linkonce_odr` only when a single
shared copy is actually needed, and never for names a linked runtime could
also define. When adding a runtime symbol, grep the backends' emitted IR for
that name. Test the combination that triggers the link, not just each piece.
