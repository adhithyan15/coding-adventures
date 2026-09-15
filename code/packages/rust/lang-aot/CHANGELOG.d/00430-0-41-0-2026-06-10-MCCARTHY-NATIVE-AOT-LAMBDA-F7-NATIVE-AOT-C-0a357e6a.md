## 0.41.0 — 2026-06-10 — McCarthy **native AOT lambda** (F7) — **NATIVE AOT COMPLETE F1–F7** (LANG77 / W14b)

No `lang-aot` source change — the fix is in `aarch64-backend` 0.9.0 + `x86_64-backend`
0.11.0 (`lispy_to_exit_code` added to `V1_BUILTINS`), which the native path already
drives. `tests/macos_native_lisp.rs` gains `mccarthy_lambda_runs_natively_on_macos`:
`((LAMBDA (X) X) 5)`→5, `((LAMBDA (X) (CAR X)) (CONS 7 9))`→7,
`((LAMBDA (X Y) (EQ X Y)) 3 3)`→1, `((LAMBDA (X) (ATOM X)) 7)`→1,
lambda-with-`COND`-body→100/200. **Native AOT is now McCarthy-complete (F1–F7)** — the
seventh backend, after VM/WASM/JVM/CLR/BEAM/LLVM. Only the JIT (W15) remains.

