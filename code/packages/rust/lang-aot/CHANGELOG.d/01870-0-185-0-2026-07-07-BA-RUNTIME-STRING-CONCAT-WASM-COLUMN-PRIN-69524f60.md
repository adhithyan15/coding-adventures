## 0.185.0 — 2026-07-07 — BA runtime string CONCAT: WASM column — `PRINT A$ + B$` on ALL SEVEN backends

The final column. The `PRINT A$ + B$` runtime-concat matrix cell (stdin `"OK\n!\n"`
→ `OK!`) now runs on **all seven backends** `[NativeAot, Llvm, Wasm, Jvm, Clr, Vm,
Jit]`.

- **WASM** (`iir-to-wasm` 0.32.0): the concat is built entirely in wasm — bump-allocate
  a `[i32 len][bytes]` block, write the header, and splice both operands' bytes with two
  `memory.copy` instructions (no scratch locals; each length re-read via `i32.load`).
- **Executor** (`wasm-execution` 0.5.0): gained a `0xFC` bulk-memory decoder + a
  `memory.copy` handler backed by `LinearMemory::copy` (bounds-checked, overlap-safe).
- Matrix cell backends → `[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit]`; both guard tests
  pass. WASM run-verified in-process locally.

This completes the E4-dyn BASIC runtime string-concatenation arc across all three PRs
(Jvm/Clr/Vm/Jit foothold → NativeAot/LLVM → WASM).

