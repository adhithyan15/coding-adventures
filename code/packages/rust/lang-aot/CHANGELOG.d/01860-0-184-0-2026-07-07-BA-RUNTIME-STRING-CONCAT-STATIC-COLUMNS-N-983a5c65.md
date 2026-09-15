## 0.184.0 — 2026-07-07 — BA runtime string CONCAT: static columns (NativeAot + LLVM)

The `PRINT A$ + B$` runtime-concat matrix cell (stdin `"OK\n!\n"` → `OK!`) gains its
two **static** columns, so it now runs on `[NativeAot, Llvm, Jvm, Clr, Vm, Jit]` —
6 of 7 backends (WASM remains). Both static backends carried a `str_concat` as a
literal fold only; each now routes a non-foldable concat to the runtime helper
`__twig_str_concat(a, b)`:

- **NativeAot** (twig-aot 0.30.0): runtime `str_concat` → `call_builtin "str_concat"`;
  aarch64 0.22.0 / x86_64 0.23.0 add `str_concat` to `V1_BUILTINS` (same shape as
  `str_eq`, no new codegen). aarch64 run-verified locally; x86_64 on CI.
- **LLVM** (iir-to-llvm 0.33.0): runtime `str_concat` → `call i64 @__twig_str_concat`
  + declare guard; literal fold retained. Via clang.
- **Harness**: `PRINT_RUNTIME_C` gains a malloc-backed `__twig_str_concat` (reads both
  `[i64 len][bytes]` headers, joins), and `run_llvm`'s link condition now also fires
  when the `.ll` references `@__twig_str_concat`.

