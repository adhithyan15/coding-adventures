## 0.104.0 — 2026-06-26 — BASIC `GOSUB`/`RETURN` matrix proof + BASIC-putchar harness fix

- **LANG-FULL BA1** — two executed Dartmouth BASIC `GOSUB`/`RETURN` programs in
  `tests/lang_matrix.rs`: `GOSUB 100` twice sharing one `RETURN` ⇒ stdout `919`
  (proves the same `RETURN` resumes at the dynamically most-recent `GOSUB`), and a
  nested `GOSUB` ⇒ `876` (LIFO across depth > 1). Run on six backends
  (native/LLVM/JVM/CLR/VM/JIT); Wasm excluded pending the BA1-WASM iir-to-wasm
  dispatch-loop fix (the computed-`goto` is irreducible → runtime `StackUnderflow`).
- **Test-harness fix (BASIC putchar print model).** BA2 switched BASIC `PRINT` to
  the `putchar` builtin, but the per-backend host wiring still assumed `print_i64`,
  leaving origin/main's BASIC matrix red on a fresh build: JVM compiled
  `env.BasicRuntime` (println) instead of `env.BFRuntime` (putchar) → empty output;
  VM/JIT/Wasm didn't `.trim()` the putchar byte stream → `"42\n"` vs `"42"`. Fixed
  `run_jvm`/`run_vm`/`run_jit`/`run_wasm` so every backend runs BASIC's putchar
  output (same harness fix as the focused PR). Both matrix guards pass on all 7
  backends for the existing BASIC cells.

