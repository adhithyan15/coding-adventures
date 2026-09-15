## 0.27.0 — 2026-06-10 — McCarthy → **BEAM (Erlang VM)** run-foundation (scalar) (LANG77 / W9a)

Adds `compile_source_to_beam` — the **fourth** managed `--emit` target and the
first on the **Erlang VM**. Source → IIR → `concretize_scalar_any_for_beam`
(scalar `any` → `i64`; the BEAM has native arbitrary-precision integers) →
`iir-to-beam` → `encode_beam` (a `.beam` module exporting `main/0`). **Scalar
McCarthy programs emit a `.beam` that RUNS** — verified by running it on a real
`erl` (OTP 28): `42`→42, `0`→0, `7`→7; Twig `42` too. Adds
`LangAotError::BeamBackendError`. BEAM uses the native **Erlang-terms** value
model (integers/atoms/list cells), not the structural uniform-reference model of
WASM/JVM/CLR — so its cons/symbol/lambda lowering (W9+) is its own.

