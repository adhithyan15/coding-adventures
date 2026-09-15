## 0.42.0 — 2026-06-10 — McCarthy on the **universal JIT** (F1–F6) (LANG77 / W15a)

Adds `run_mccarthy_on_jit(source)` + the `jit_lisp` module: McCarthy now runs on
`jit-core`'s `GenericCirJit` — the eighth and final backend. The JIT dispatches
`call_builtin "lispy_*"` to Rust callbacks (not native `__twig_lispy_*` calls like
the AOT/LLVM path), so the lisp ops are registered against the shared `lispy-runtime`
crate (the C runtime's Rust twin — identical `u64` tagged-word model). A `LispyValue`
rides inside `Value::Int` as its bit pattern; `unbox_int`/`truthy` are derived from
`LispyValue::as_int`/`is_truthy` (existing primitives, not duplicated). New deps:
`vm-core`, `lispy-runtime`. New error variant `LangAotError::JitBackendError`.
Verified by RUNNING (`tests/jit_mccarthy.rs`): `(CAR (CONS 7 9))`→7, `(ATOM 7)`→1,
`(EQ 7 7)`→1, nested `COND`→44, `(EQ (QUOTE A) (QUOTE A))`→1. Lambda (F7) is W15b —
the VM's user-`call` path needs work first.

