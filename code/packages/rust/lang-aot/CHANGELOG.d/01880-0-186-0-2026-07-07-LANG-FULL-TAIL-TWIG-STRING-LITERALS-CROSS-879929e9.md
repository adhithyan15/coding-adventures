## 0.186.0 — 2026-07-07 — lang-full tail: Twig string literals crossing a call now run on WASM

Four Twig/McCarthy-lisp string cells that pass a string LITERAL across a function
boundary (`(define (strlen s) (string-length s)) (strlen "HELLO")` and variants —
direct arg, top-level `define`, `let` binding) gain their **WASM** column, going from
5 → 6 backends `[NativeAot, Wasm, Jvm, Clr, Vm, Jit]`. They previously returned 72
(=`'H'`) on WASM because the callee read the raw literal's bytes as a length; fixed by
`iir-to-wasm` 0.33.0 promoting a `str_const` literal used as a call argument to a
length-prefixed runtime block. (Bumps the Cargo version to 0.186.0, reconciling it with
the 0.185.0 CHANGELOG entry left by #7770.)

The remaining lang-full string tail — the same cells' **LLVM** column, and the cells
that pass a `str_concat`/`substring` *result* across a call — are follow-ups.

