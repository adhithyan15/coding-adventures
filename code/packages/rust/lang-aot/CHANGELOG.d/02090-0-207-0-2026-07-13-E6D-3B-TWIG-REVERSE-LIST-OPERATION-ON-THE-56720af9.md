## 0.207.0 - 2026-07-13 — E6d-3b: Twig `reverse` list operation on the code-gen backends

One matrix cell proves the fourth list operation, `reverse`:

- `(car (reverse (list 1 2 42)))` → 42

on `[NativeAot, Llvm, Wasm, Jvm, Clr]`. `reverse (list 1 2 42)` builds `(42 2 1)`;
`car` = 42. `reverse` is lowered by `iir-builtin-lowering` 0.27.0's `lower_list_ops`
to a **nil-seeded** call to a synthesized *tail-recursive accumulator* helper
`__dyn_list_reverse`:

```
reverse(a)          = __dyn_list_reverse(a, nil)      # nil seed at the call site
__dyn_list_reverse(a, acc) = if null?(a) then acc
                             else __dyn_list_reverse(cdr(a), cons(car(a), acc))
```

Consing each element onto the accumulator's front reverses the order. The call
site seeds the empty accumulator as `const 0 : ref<LispyPair>` (the exact nil
sentinel the `list` desugar / `make_nil` emit). Like `append` there is no index,
so no unbox/box — the recursion reuses `null?`/`car`/`cdr`/`cons` (E6d-1).
Run-verified locally on WASM (in-process) + real dotnet CLR; native/LLVM/JVM via
CI. `assoc` follows the same pattern.

