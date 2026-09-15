## 0.21.0 — 2026-06-09 — McCarthy → **JVM** run-foundation (scalar) (LANG77 / W3a)

Adds `compile_source_to_jvm` / `compile_file_to_jvm` — the second *managed*
`--emit` target. Source → IIR → `concretize_scalar_any_for_jvm` (scalar `any`/
`i64` → JVM `i32`) → `iir-to-jvm-class-file` → a serialized `.class`. **Scalar
McCarthy programs emit a class that RUNS** — verified end-to-end by parsing the
emitted bytes and running the entry method on the in-repo `jvm-simulator` (zero
external `java`, mirroring how the wasm path uses `wasm-runtime`): `42` → 42,
`0` → 0, `7` → 7; Twig `42` too. The cons/symbol/lambda uniform-`Object` value
model (the JVM replication of the WASM structural passes) is W3b+.

