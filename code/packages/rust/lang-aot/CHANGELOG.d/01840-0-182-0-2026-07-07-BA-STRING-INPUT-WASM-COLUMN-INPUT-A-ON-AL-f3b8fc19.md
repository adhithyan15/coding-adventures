## 0.182.0 — 2026-07-07 — BA string INPUT: WASM column — `INPUT A$` on ALL SEVEN backends

The final column. The `INPUT A$` matrix cell now runs on **all seven backends**
`[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit]` (stdin `"OK"` → `OK`).

- **WASM** (`iir-to-wasm` 0.31.0): `call_builtin "input_str"` bump-allocates a
  `[i32 len][256 bytes]` block and calls `env.__input_str(block, max) -> ()`.
- **Harness** (`run_wasm`): new `InputStrFunc` host resolves that import — it drains
  one line from the shared stdin buffer and writes the `[i32 len][bytes]` block into
  wasm linear memory (`store_i32` header + `store_i32_8` bytes), capped at `max`.
  Registered next to `InputI64Func`/`PrintStrFunc`. Run-verified in-process locally.

This completes the E4-dyn BASIC string `INPUT A$` arc across all four PRs (VM/JIT →
JVM/CLR → native/LLVM → WASM).

