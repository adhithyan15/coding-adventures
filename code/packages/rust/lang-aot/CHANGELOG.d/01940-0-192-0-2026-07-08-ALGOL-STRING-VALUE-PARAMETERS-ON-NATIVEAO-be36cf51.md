## 0.192.0 — 2026-07-08 — ALGOL string value-parameters on NativeAot+LLVM+WASM → **matrix 100% (159/159 on all 7 backends)**

The two ALGOL `string`-value-parameter cells (`echo('HELLO')` with an inline-literal
actual, and `say(msg)` with a named outer-block string slot as the actual — each
`value s; string s; print(s)`) gain their **NativeAot**, **LLVM**, and **WASM** columns.
No backend change was needed: the E4-dyn runtime-string work (E4d-2b LLVM / E4d-3b WASM /
E4d-4 native) already gave `print_str` a runtime path that reads the `[len][bytes]` block
header at run time for any string lacking a compile-time-length entry — which is exactly
a string parameter. The cells' exclusion comments predated that work; run-verified here
(native via a real executable + stdout capture, LLVM via clang, WASM via `wasm-runtime`).

**With this every cell in the conformance matrix (159/159) runs on all 7 backends** —
NativeAot, LLVM, WASM, JVM, CLR, VM, JIT.

