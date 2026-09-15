## 0.187.0 — 2026-07-07 — lang-full tail: Twig string literals crossing a call now run on ALL 7 backends

The 4 Twig/McCarthy-lisp cells that pass a string LITERAL across a function boundary
(`(define (strlen s) (string-length s)) (strlen "HELLO")` and variants) gain their
**LLVM** column, reaching **all 7 backends** `[NativeAot, Llvm, Wasm, Jvm, Clr, Vm,
Jit]` (they were 6/7 after 0.186.0's WASM fix). `iir-to-llvm` 0.34.0 `ptrtoint`s a
`str_const` literal's global pointer to an i64 handle at the call site, so the callee's
runtime `str_len` reads the length header. These 4 cells are now fully at all 7 backends.

The remaining 3 cells — those passing a `str_concat`/`substring` RESULT (not a
`str_const`) across a call — are the last of the string tail.

