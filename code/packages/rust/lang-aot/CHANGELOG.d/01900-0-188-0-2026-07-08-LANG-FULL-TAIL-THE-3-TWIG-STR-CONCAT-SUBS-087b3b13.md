## 0.188.0 — 2026-07-08 — lang-full tail: the 3 Twig `str_concat`/`substring`/`string=?`-across-call cells now run on LLVM

The last 3 Twig/McCarthy-lisp string cells — passing a `str_concat`/`substring` RESULT
across a call, or comparing two runtime strings with `string=?` — gain their **LLVM**
column: `[NativeAot, Llvm, Jvm, Clr, Vm, Jit]` (6/7; WASM is the final follow-up).

- The `substring`/`let*`-concat cells' folded results are `@__twig_str` globals already
  `ptrtoint`'d by `lower_call` (0.34.0).
- The `string=?` cell needed `iir-to-llvm` 0.35.0's new runtime `str_eq` path (calls
  `@__twig_str_eq`). The `run_llvm` test harness gains a malloc-free `__twig_str_eq`
  shim + links it when the `.ll` references the symbol.

With this, the only lang-full string-tail work left is these 3 cells' **WASM** column
(promoting a folded `str_concat`/`str_slice` result to a runtime block).

