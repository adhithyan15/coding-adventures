## 0.205.0 - 2026-07-12 — E6d-3b: Twig `list-ref` list operation on the code-gen backends

One matrix cell proves the second list operation, `list-ref` (an index walk):

- `(list-ref (list 10 20 42) 2)` → 42

on `[NativeAot, Llvm, Wasm, Jvm, Clr]`. Like `length`, `list-ref` is lowered by
`iir-builtin-lowering` 0.25.0's `lower_list_ops`, which rewrites
`call_builtin "list-ref" lst n` to a call to a synthesized recursive helper
`__dyn_list_ref` and injects it once. The helper reuses `car`/`cdr` (E6d-1):
`if ni == 0 then car(lst) else list-ref(cdr(lst), ni - 1)`.

Design note — **the index is a boxed lisp value.** The lisp calling convention
is uniform-anyref (`dyn_repr_structural` on the managed backends / `lower_dyn_repr`
on native box *every* argument to a lisp function), so a raw-`i64` index param
faults at the call boundary (`expected i64, got I32(2)`). The helper takes
`n : ref<any>` and unboxes it once; the index test (`ni == 0` → raw `cmp_eq :
bool` feeding `jmp_if_false`) and decrement (`ni - 1` → raw `sub : i64`,
re-boxed for the recursive call) are then plain typed machine ops. Its return is
always a `car` result / the recursive call — cleanly `ref<any>`. This mirrors the
explicit-`box`/`unbox` shape the `length` helper already uses, proven-portable on
all five columns. Run-verified locally on WASM (in-process) + real dotnet CLR;
native/LLVM/JVM via CI. `append`/`reverse`/`assoc` follow the same pattern.

