## 0.195.0 — 2026-07-10 — E6d-1: Twig dynamic cons/car/cdr on the code-gen backends

First slice of **E6 layer 2** (general dynamic dispatch — spec
`code/specs/lang-full-e6-dispatch.md`). Two matrix cells prove Twig's first
genuinely *dynamic* value runs on the code-gen backends:

- `(car (cons 42 0))` → 42
- `(car (cdr (cons 1 (cons 42 0))))` → 42 (nested, multi-cell pointer chasing)

on `[NativeAot, Llvm, Wasm, Jvm, Clr]`. **Zero production code** — the surveyed
fact held: the shared `iir-builtin-lowering` heap passes (`lower_heap_builtins` →
`lower_lisp_repr_structural`), which run for *every* source language, already
lower Twig's `call_builtin "cons"/"car"/"cdr"` to the `alloc`/`field_load`/
`field_store` heap-object family over the uniform boxed `ref<any>` value that
McCarthy Lisp already runs on all five code-gen backends (WASM `anyref` +
`$LispyPair`, JVM `Object[]`, CLR `object[]`, LLVM tagged-i64 + `__twig_lispy_*`
runtime, native). Run-verified locally on **WASM** (in-process) and **real dotnet
CLR**; native/LLVM/JVM via CI.

The generic `Vm`/`Jit` columns run `vm-core` typed IIR (no `ref<any>`/`alloc`), so
dynamic Twig cells list the 5 code-gen columns; `twig-vm` is the off-matrix
interpreter reference. Turns the stale `TW3 ☐` (cons core) into a guardrailed
proof. Remaining E6 layer-2 slices: dynamic arithmetic, list ops, symbols,
records/unions, closures-on-WASM, dynamic globals.

