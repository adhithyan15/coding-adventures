## 0.200.0 - 2026-07-11 (DVAL01-2: rename IIR builtin names lispy_* -> dyn_* + passes)

DVAL01-2: the lang-aot wiring, `jit_lisp.rs`, and the LLVM/JIT/metacircular/
conformance integration tests move to the `dyn_*` IIR builtin names and the
renamed `dyn_repr`/`dyn_repr_structural` passes. Verified: the McCarthy-lisp
cells stay green across VM/JIT/LLVM/native (native now emits the correct
`__dyn_*` runtime symbols); cross-backend agreement is preserved. Pure rename
of the builtin-name surface + the native emit fix (see aarch64/x86_64-backend).

