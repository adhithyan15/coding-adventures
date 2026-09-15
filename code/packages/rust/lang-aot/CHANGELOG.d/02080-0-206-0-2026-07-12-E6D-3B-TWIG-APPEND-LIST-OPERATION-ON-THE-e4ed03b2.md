## 0.206.0 - 2026-07-12 — E6d-3b: Twig `append` list operation on the code-gen backends

One matrix cell proves the third list operation, `append` (a list *rebuild*):

- `(car (cdr (append (list 1 42) (list 3))))` → 42

on `[NativeAot, Llvm, Wasm, Jvm, Clr]`. `append (list 1 42) (list 3)` builds
`(1 42 3)`; `car (cdr …)` = 42, proving the rebuilt spine is a genuine cons chain.
`append` is lowered by `iir-builtin-lowering` 0.26.0's `lower_list_ops`, which
rewrites `call_builtin "append" a b` to a synthesized recursive helper
`__dyn_list_append` and injects it once:

```
__dyn_list_append(a, b) = if null?(a) then b else cons(car(a), append(cdr(a), b))
```

Unlike `list-ref` there is no index, so no unbox/box — `a`/`b`, `car(a)`, and the
recursive result are all lisp references. Its one new op versus `length`/`list-ref`
is the `cons` in the recursive arm (the E6d-1 heap builtin, rewritten to
`alloc`/`field_store` for the injected helper by the same head-of-heap-lowering
pass). Run-verified locally on WASM (in-process) + real dotnet CLR; native/LLVM/JVM
via CI. `reverse`/`assoc` follow the same pattern.

