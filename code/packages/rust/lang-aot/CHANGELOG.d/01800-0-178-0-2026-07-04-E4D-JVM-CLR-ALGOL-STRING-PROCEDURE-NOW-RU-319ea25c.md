## 0.178.0 — 2026-07-04 — E4d-JVM/CLR: ALGOL string procedure now runs on ALL SEVEN backends

The ALGOL string-procedure matrix cell (`print(pick(1))` where `pick` returns a
runtime, branch-selected string) gains the final two columns — **`Jvm`** and
**`Clr`** — so it now runs on **all seven backends** (NativeAot, Llvm, Wasm, Jvm,
Clr, Vm, Jit), all agreeing on `"HI"`. A `string procedure` — the first E4-dyn
*frontend* feature — is now fully portable.

- **JVM** (`iir-to-jvm-class-file` 0.28.0): a `str` value is already a
  `java.lang.String`; the validator now accepts `str` on `call`/`ret` (the same
  fix as WASM E4d-3b). Verified on real `java`.
- **CLR**: already compiled and ran the string procedure (a `str` is a
  `System.String`, and its validator/lowering already accepted `str` call/ret) —
  it only needed adding to the cell. Verified via the CLR simulator.

Both matrix guard tests pass.

