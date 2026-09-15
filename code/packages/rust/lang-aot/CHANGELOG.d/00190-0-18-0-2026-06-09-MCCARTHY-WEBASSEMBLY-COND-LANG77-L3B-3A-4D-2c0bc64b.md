## 0.18.0 — 2026-06-09 — McCarthy → WebAssembly, **`COND`** (LANG77 / L3b-3a-4d)

`compile_source_to_wasm` now compiles McCarthy's `COND` conditional with correct
lisp-truthiness. The structural pass wraps a lisp-value clause guard with
`not(is_null(...))`, so an integer atom (even `0`) is true and only `nil` is
false; predicate guards (`pair?`/`EQ`) test directly. The control flow already
lowered. Verified end-to-end: `(COND ((ATOM 5) 7) (5 9))` → 7,
`(COND ((ATOM (CONS 1 2)) 7) (5 9))` → 9, `(COND (0 7) (5 9))` → 7 (0 truthy!),
`(COND ((ATOM (CONS 1 2)) 7))` → nil (exit 0). **This completes the McCarthy
core — cons, ATOM/pair?, EQ, and COND — on the wasm backend.**

