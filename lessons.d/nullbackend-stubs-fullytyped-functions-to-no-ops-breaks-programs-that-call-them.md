# NullBackend stubs FullyTyped functions to no-ops — breaks programs that call them

`jit_smoke.rs` used `jit_core::NullBackend`, whose `compile()` returns
`Some(sentinel)` and `run()` returns `Value::Null` — i.e. it "compiles" every
function to a **no-op binary**. Before BA2 this was harmless: BASIC's `main` was
the only function and `execute_with_jit` Phase-2 interprets `main` directly. BA2
made `PRINT` lower to a `call` of FullyTyped helper functions
(`__basic_print_int`/`__basic_print_uint`), which Phase-1 **eagerly compiles** —
with NullBackend they became no-op binaries, so `PRINT 42` emitted only the
trailing newline (`"\n"`, no digits). It passed locally but failed on CI
(environment-dependent whether the cached no-op was consulted). The sibling test
`jit_real_backend.rs` (BasicCirJit, whose `compile` returns `None` for the
helpers' `call`/`putchar` → interpreter runs them) PASSED on CI — proving the
interpreter executes the recursive helpers correctly and that `compile → None`
is the working pattern. LESSON: `NullBackend` is only safe for programs whose
output doesn't depend on a *called* function's side effects; once a frontend
emits cross-function `call`s, use a backend whose `compile` returns `None`
(defer-to-interpreter) so nothing is stubbed. When a JIT test passes locally but
fails on CI with missing output, suspect eager-compilation of FullyTyped
functions to no-op binaries.
