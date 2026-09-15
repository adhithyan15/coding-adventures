## 0.17.0 — 2026-06-08 — McCarthy → WebAssembly, **`EQ`/`equal?`** (LANG77 / L3b-3a-4c)

`compile_source_to_wasm` now compiles McCarthy's `EQ` (atom equality): the atoms
are boxed as `i31ref` by the structural pass, and `iir-to-wasm` lowers `equal?`
to unbox-both + `i32.eq`. **`(EQ 5 5)` → 1, `(EQ 5 6)` → 0**, and the compared
values may be computed (`(EQ (CAR (CONS 3 4)) 3)` → 1). Atom equality only
(McCarthy `eq`); deep structural `equal` over cons cells is later.

