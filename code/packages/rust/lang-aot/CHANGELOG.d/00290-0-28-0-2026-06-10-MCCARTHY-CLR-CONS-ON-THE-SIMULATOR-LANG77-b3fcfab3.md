## 0.28.0 — 2026-06-10 — McCarthy → **CLR cons** on the simulator (LANG77 / W6b)

`compile_source_to_cil_artifact` now runs the **managed value-model pipeline**
(the same `lower_heap_builtins` + `intern_symbols_structural` +
`lower_lisp_repr_structural` the wasm/JVM paths use), so McCarthy **cons** runs on
the CLR: `(CAR (CONS 7 9))` → 7, `(CDR (CONS 7 9))` → 9, nested cons too, on the
object-capable in-repo `clr-simulator` (W6b-1). The structural passes emit
backend-agnostic `box`/`unbox`/`alloc`/`field_*`; `iir-to-cil-bytecode` 0.8.0
lowers them to `box [int32]`/`unbox.any` + `object[]` cells (where wasm uses
`i31ref`/`$LispyPair` and the JVM `Integer`/`Object[]`). New `tests/cil_cons.rs`.

