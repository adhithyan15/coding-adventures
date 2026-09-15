## 0.220.35 - 2026-08-03 (fix `llvm_lambda.rs` undefined GC symbols)

`tests/llvm_lambda.rs` linked only `dynval_runtime.c`, whose `__dyn_cons` calls
`__gc_alloc_kind`/`__gc_register_kind` — symbols only the `gc-core-capi`
staticlib defines. Every McCarthy lambda test failed to link ("Undefined
symbols ... ___gc_alloc_kind") on any host with `clang` available. Adds
`mod common;` and `common::gc_link_args()` to the clang invocation, mirroring
the working `e6d6_llvm_records.rs` pattern. Confirmed the fix is real: reverted
locally, the same undefined-symbol linker error reproduces exactly.

