## 0.85.0 — 2026-06-14 — Twig top-level value `define` runs cross-backend (LANG-FULL TW2)

`tests/lang_matrix.rs` gains an executed **top-level value `define`** program:

```scheme
(define x 40) (define y 2) (+ x y)
```

⇒ exit **42**, running across native / LLVM / WASM / JVM / CLR / VM / JIT.
Previously a value `define` lowered to `call_builtin "global_set"` (and reads to
`global_get`), `type_hint = "any"` — rejected by every code-gen backend
validator, so top-level constants ran only on the VM. `twig-ir-compiler`
(0.24.0) adds a small escape analysis: a value `define` not captured by any
lambda is read only from `main`, so its statically-typed value stays in a `main`
register and reads return it directly — no `call_builtin` survives. A
lambda-captured value (or a forward reference) still uses the host global table,
unchanged.

