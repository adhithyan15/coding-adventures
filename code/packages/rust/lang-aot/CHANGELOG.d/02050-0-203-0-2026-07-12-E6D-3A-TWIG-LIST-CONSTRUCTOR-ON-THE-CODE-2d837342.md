## 0.203.0 - 2026-07-12 — E6d-3a: Twig `list` constructor on the code-gen backends

Two matrix cells prove the `list` constructor as Twig's next dynamic capability:

- `(car (list 42 1 2))` → 42
- `(car (cdr (list 1 42 3)))` → 42 (list + `cdr` reaches the second element)

on `[NativeAot, Llvm, Wasm, Jvm, Clr]`. `list` is pure sugar over `cons`, so it
needed **no new backend op**: `iir-builtin-lowering` 0.23.0's new
`desugar_list_in_function` expands `call_builtin "list"` → a nil `const` + a
right-to-left `cons` chain at the head of *both* heap-lowering entry points
(`lower_heap_builtins` managed + `lower_heap_builtins_runtime` native/LLVM), so the
list rides the exact E6d-1 cons path on all five code-gen backends — no lang-aot
pipeline change, no `call_builtin` allowlist entry. Run-verified locally on WASM +
real dotnet CLR; native/LLVM/JVM via CI. List *operations* (`length`/`list-ref`/
`append`/`reverse`) are E6d-3b (they walk the cons chain — a helper, not a desugar).

