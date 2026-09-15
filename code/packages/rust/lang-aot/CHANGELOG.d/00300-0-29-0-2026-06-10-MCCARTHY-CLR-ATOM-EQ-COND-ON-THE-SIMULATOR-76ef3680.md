## 0.29.0 — 2026-06-10 — McCarthy → **CLR ATOM/EQ/COND** on the simulator (LANG77 / W7)

The CLR backend now runs McCarthy's primitive predicates (F3–F5). The same
managed structural pipeline `compile_source_to_cil_artifact` already runs (no
driver change) emits `pair?`/`not`/`equal?`/`jmp_if`; `iir-to-cil-bytecode` 0.9.0
+ `clr-simulator` 0.3.0 (new `isinst`/`xor` + ref-aware compares) execute them.
New `tests/cil_predicates.rs`: `(ATOM 7)`→1, `(ATOM (CONS 1 2))`→0, `(EQ 7 7)`→1,
`(EQ 7 8)`→0, `(COND ((ATOM 7) 100) ((ATOM 8) 200))`→100, and fall-through→200.

