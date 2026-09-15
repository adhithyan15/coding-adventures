## 0.220.1 - 2026-07-17 (#118b-2b: tests link gc-core-capi, not the retired twig_gc.c)

Test-only change tracking the retirement of `twig-aot/runtime/twig_gc.c`. The
LLVM/native GC-allocating integration tests (`e6d2b_dynamic_arith`,
`e6d6_llvm_records`, `e6d6b_union_match_tagged`, `e6d7a_wasm_closures`,
`llvm_cons`/`llvm_cond`/`llvm_predicates`/`llvm_symbols`, and the `@__dyn_` cells in
`lang_matrix`) used to hand `clang` the three C runtime files including `twig_gc.c`.
That file is gone; a new `tests/common/mod.rs` helper (`gc_link_args`) builds
`gc-core-capi`'s release staticlib and returns it plus the platform link libraries
(`-lpthread -ldl` on Linux) in its place. No production-source or ABI change.

