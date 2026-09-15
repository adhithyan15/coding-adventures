## 0.189.0 — 2026-07-08 — lang-full tail: 2 of the 3 Twig concat/substring-across-call cells now run on WASM

The `substring`-result and `let*`-`str_concat`-result cells gain their **WASM** column
(`iir-to-wasm` 0.34.0's folded-result runtime-block promotion), so they now run on **all
7 backends**. Both previously produced exit `72` on WASM (`'H'` — the callee read the
first data byte as the string length) and were excluded from the WASM column.

The third cell (`string=?` over two runtime string params) still lacks WASM: `str_eq`
on WASM has no runtime path yet (it requires both operands be direct `str_const` locals).
That is the final lang-full string-tail item, to follow in a separate PR.

