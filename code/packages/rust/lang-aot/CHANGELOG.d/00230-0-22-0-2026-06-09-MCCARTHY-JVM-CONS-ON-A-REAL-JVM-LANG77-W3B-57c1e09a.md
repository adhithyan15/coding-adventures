## 0.22.0 — 2026-06-09 — McCarthy → **JVM cons** on a real JVM (LANG77 / W3b)

`compile_source_to_jvm` now runs the **managed value-model pipeline** (the same
`lower_heap_builtins` + `intern_symbols_structural` + `lower_lisp_repr_structural`
the wasm path uses), so McCarthy **cons** runs on the JVM: `(CAR (CONS 7 9))` →
7, `(CDR (CONS 7 9))` → 9, nested cons too. The structural passes emit
backend-agnostic `box`/`unbox`/`alloc`/`field_*`; `iir-to-jvm-class-file` lowers
them to `Integer.valueOf`/`intValue` + `Object[]` cells (where wasm uses
`i31ref`/`$LispyPair`) — the reusable primitive a future lisp inherits. Adds
`compile_source_to_jvm_class` (returns the `JvmClassFile` pre-serialization, so a
caller can inject a `main` launcher). Verified by **running on a real `java`**
(Temurin 21; the cons cells are `Object[]` the in-repo `jvm-simulator` can't
execute) — see `tests/jvm_cons.rs`.

