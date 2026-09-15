## 0.204.0 - 2026-07-12 — E6d-3b: Twig `length` list operation on the code-gen backends

Two matrix cells prove the first list *operation* (a cons-chain walk, unlike the
E6d-3a `list` constructor):

- `(+ (length (list 1 2 3)) 39)` → 42 — `length` = 3, composed with dynamic `+`,
  proving it returns a genuine boxed lisp value.
- `(null? (list))` → 1 — the empty list is nil; the direct regression guard for
  the WASM nil-const fix.

on `[NativeAot, Llvm, Wasm, Jvm, Clr]`. `length` is lowered by `iir-builtin-lowering`
0.24.0's new `lower_list_ops`, which rewrites `call_builtin "length"` to a call to a
synthesized recursive `__dyn_list_length` helper (a proper lisp function: `null?`/
`cdr` + dynamic `+`, all already lowered by E6d-1/E6d-2). This also required fixing
the **WASM nil const**: `const 0 : ref<LispyPair>` now emits `ref.null`
(iir-to-wasm 0.38.0) so `is_null`/`null?` detects the terminator — it was
`i32.const 0`, so a walk overran the list end into `struct.get` on an i32. (CLR
already lowered nil to `ldnull`; car/cdr never touch the nil tail, so E6d-1/E6d-3a
were unaffected.) Run-verified locally on WASM + real dotnet CLR; native/LLVM/JVM
via CI. `list-ref`/`append`/`reverse` follow the same helper pattern.

