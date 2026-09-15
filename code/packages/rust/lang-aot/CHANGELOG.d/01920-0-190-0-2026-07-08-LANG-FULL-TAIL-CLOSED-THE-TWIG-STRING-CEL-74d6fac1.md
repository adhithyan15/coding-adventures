## 0.190.0 — 2026-07-08 — lang-full tail CLOSED: the Twig `string=?` cell now runs on WASM (all 3 on all 7 backends)

The last remaining lang-full string-tail cell — `string=?` over two runtime string
handles — gains its **WASM** column via `iir-to-wasm` 0.35.0's in-module `$__str_eq`
helper. With this, all 3 Twig/McCarthy-lisp runtime-string cells
(`substring`-result, `let*`-`str_concat`-result, `string=?`) run on **all 7 backends**
— the lang-full string tail is complete.

