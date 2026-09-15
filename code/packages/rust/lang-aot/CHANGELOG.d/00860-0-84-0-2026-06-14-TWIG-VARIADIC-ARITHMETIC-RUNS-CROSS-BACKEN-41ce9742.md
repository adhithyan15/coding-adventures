## 0.84.0 — 2026-06-14 — Twig variadic arithmetic runs cross-backend (LANG-FULL TW1)

`tests/lang_matrix.rs` gains an executed **n-ary Twig arithmetic** program:

```scheme
(+ 10 20 12)
```

⇒ exit **42**, running across native / LLVM / WASM / JVM / CLR / VM / JIT.
`twig-ir-compiler` (0.23.0) folds an all-`i64` arithmetic call (`+`/`-`/`*`/`/`)
into a left-associated chain of typed binary CIR ops (`r1 = add 10,20; r2 = add
r1,12`). Before TW1 only the binary `(+ a b)` form lowered to a typed `add`;
three-or-more-argument calls fell back to `call_builtin "+"` (`type_hint =
"any"`), which every code-gen backend validator rejects — so this is the first
variadic Twig arithmetic to run anywhere but the dynamic VM/JIT path.

