## 0.14.0 — 2026-06-05 — McCarthy → WebAssembly, scalar (LANG77 / L3b-3a-2)

Adds `compile_source_to_wasm` / `compile_file_to_wasm` — the first of the
modern *managed* `--emit` targets. The pipeline runs the **structural** heap
lowering (`iir_builtin_lowering::lower_heap_builtins`) then
`iir-to-wasm`'s WasmGC backend + `encode_module`.

**Scope: scalar programs.** The managed backends are *typed* and reject the
polymorphic `"any"` lisp value (a `LispyValue`), so a new
`concretize_scalar_any_for_wasm` pass retypes `"any"`→`"i64"` for any function
with **no heap/reference ops** (every value there is a machine integer). Cons/
symbol programs need the boxed-`anyref` value model — a follow-up slice — and
fail cleanly with a `WasmBackendError` for now.

**Verified end-to-end, zero-external-dep:** the new tests *run* the emitted
module on the in-repo `wasm-runtime` (a dev-dependency) and assert the result —
McCarthy `42` → a `.wasm` whose `main` returns `i64 42`; a Twig `42` runs the
same path (reusability); a cons program is a clean error. New
`WasmBackendError` variant.

