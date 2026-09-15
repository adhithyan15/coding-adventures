## 0.181.0 — 2026-07-07 — BA string INPUT: static columns (NativeAot + LLVM)

The `INPUT A$` matrix cell (`10 INPUT A$ / 20 PRINT A$ / 30 END`, stdin `"OK"` →
`OK`) gains its two **static** columns, so it now runs on `[NativeAot, Llvm, Jvm,
Clr, Vm, Jit]` — 6 of 7 backends (WASM remains). On the static backends a `str`
is an i64 handle to a `[i64 len][bytes]` heap block, so both columns gain a host
primitive that BUILDS such a block from the input line:

- **NativeAot** (`twig-aot` 0.29.0): `__twig_input_str()` in `twig_runtime.c`;
  the aarch64 (0.21.0) / x86_64 (0.22.0) tables add it as a 0-arg/returns-i64
  `V1_BUILTINS` entry (no codegen change). Run-verified locally on aarch64.
- **LLVM** (`iir-to-llvm` 0.32.0): `call_builtin "input_str"` → `call i64
  @__twig_input_str()`; also fixes a latent declare-guard bug that omitted the
  input helpers. The harness `run_llvm` C shim (`PRINT_RUNTIME_C`) gains a
  self-contained `__twig_input_str` (malloc-backed) and links it when the IR
  references it. Run-verified locally via real `clang`.

WASM (`env.__input_str` writing the block into linear memory) is the last column.

