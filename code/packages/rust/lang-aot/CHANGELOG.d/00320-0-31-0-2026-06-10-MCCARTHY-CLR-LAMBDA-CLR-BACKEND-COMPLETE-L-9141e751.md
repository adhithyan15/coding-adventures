## 0.31.0 — 2026-06-10 — McCarthy → **CLR lambda** — CLR backend COMPLETE (LANG77 / W8b, F7)

Lambda (F7) runs on the CLR — **completing the entire CLR backend (F1–F7)**, the
third managed backend to reach full McCarthy support after WASM and JVM.
`(LAMBDA (args…) body)` applied lowers to a CLR method `call`: the structural pass
hoists the lambda into its own method (params → `ldarg.N`), `iir-to-cil-bytecode`
0.10.0 validates+emits `call <MethodDef>` (args boxed, result `ref<any>`), and
`clr-simulator` 0.4.0 executes it via an inter-method **call-frame** model.
`((LAMBDA (X) (CAR X)) (CONS 7 9))`→7, `((LAMBDA (X Y) (EQ X Y)) 3 3)`→1, and
backward-compat (scalar/cons) verified on the simulator. New `tests/cil_lambda.rs`.

