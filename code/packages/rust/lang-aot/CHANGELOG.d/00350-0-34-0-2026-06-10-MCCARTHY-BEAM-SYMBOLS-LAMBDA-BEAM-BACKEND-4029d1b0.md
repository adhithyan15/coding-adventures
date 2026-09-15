## 0.34.0 — 2026-06-10 — McCarthy → **BEAM symbols + lambda** — BEAM backend COMPLETE (LANG77 / W11, F6–F7)

Symbols (F6) and lambda (F7) run on the BEAM — **completing the entire BEAM
backend (F1–F7)**, the FIFTH backend to reach full McCarthy support (after VM,
WASM, JVM, CLR). One-line pipeline addition: `compile_source_to_beam` now runs
`intern_symbols_structural`, so each symbol interns to a stable `i32` id
(`SYMBOL_ID_BASE = 1<<29`) — the SAME id the wasm/JVM/CLR backends assign — which
the BEAM carries as a native Erlang integer (`EQ` → `is_eq_exact`). **Lambda
needed nothing extra** — a `(LAMBDA …)` application is a method `call`, which
`iir-to-beam` already lowers natively (a BEAM fun). Verified by RUNNING on a real
`erl`: `(QUOTE A)`→536870912, `(EQ (QUOTE A) (QUOTE A))`→1,
`((LAMBDA (X) (CAR X)) (CONS 7 9))`→7, `((LAMBDA (X) (EQ X (QUOTE A))) (QUOTE A))`→1.
New `tests/beam_symbols_lambda.rs`.

