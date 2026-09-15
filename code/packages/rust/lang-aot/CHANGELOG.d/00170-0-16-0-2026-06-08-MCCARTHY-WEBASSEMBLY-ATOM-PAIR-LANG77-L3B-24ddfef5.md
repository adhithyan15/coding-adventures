## 0.16.0 — 2026-06-08 — McCarthy → WebAssembly, **`ATOM`/`pair?`** (LANG77 / L3b-3a-4b)

`compile_source_to_wasm` now compiles McCarthy's `pair?` / `ATOM` predicate. The
structural representation pass boxes the predicate's integer atom as an `i31ref`
and concretises the boolean result to `i32`; `iir-to-wasm` lowers `pair?` to
`ref.test $LispyPair` and the lisp `not` to `i32.eqz`. So `ATOM x` =
`not(pair? x)` runs: **`(ATOM 5)` → 1, `(ATOM (CONS 1 2))` → 0**. Cons and scalar
programs (McCarthy and Twig) are unaffected (regression-tested).

