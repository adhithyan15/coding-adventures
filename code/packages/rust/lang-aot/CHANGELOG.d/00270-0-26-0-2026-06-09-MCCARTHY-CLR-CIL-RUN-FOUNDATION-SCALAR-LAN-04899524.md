## 0.26.0 — 2026-06-09 — McCarthy → **CLR (CIL)** run-foundation (scalar) (LANG77 / W6a)

Adds `compile_source_to_cil_artifact` — the **third** managed `--emit` target
(after WASM and JVM). Source → IIR → `concretize_scalar_any_for_cil` (scalar
`any`/`i64` → CLR `i32`) → `iir-to-cil-bytecode`. **Scalar McCarthy programs emit
CIL that RUNS** — verified by running the entry method's IL on the in-repo
`clr-simulator` (zero external `dotnet`, mirroring how the JVM path uses
`jvm-simulator`): `42`→42, `0`→0, `7`→7; Twig `42` too. Adds
`LangAotError::ClrBackendError`. The cons/symbol/lambda uniform-`object` value
model (the CLR replication of the shared structural passes, reusing the JVM
strict-backend fixes) is W6b+.

