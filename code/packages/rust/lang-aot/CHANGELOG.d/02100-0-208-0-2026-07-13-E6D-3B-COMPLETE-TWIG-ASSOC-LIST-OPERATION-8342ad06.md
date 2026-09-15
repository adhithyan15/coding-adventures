## 0.208.0 - 2026-07-13 — E6d-3b COMPLETE: Twig `assoc` list operation on the code-gen backends

Two matrix cells prove the fifth and final E6d-3b list operation, `assoc`:

- `(cdr (assoc 2 (list (cons 1 10) (cons 2 42) (cons 3 30))))` → 42 (found)
- `(null? (assoc 9 (list (cons 1 10) (cons 2 20))))` → 1 (absent key → nil)

on `[NativeAot, Llvm, Wasm, Jvm, Clr]`. This **completes the E6d-3b list-operation
set** — `length`, `list-ref`, `append`, `reverse`, `assoc` all run on the five
code-gen backends. `assoc` searches an association list (a list of `(k . v)` cons
pairs) via a synthesized recursive helper (`iir-builtin-lowering` 0.28.0):

```
__dyn_list_assoc(key, alist) = if null?(alist) then nil
    else if key == car(car(alist)) then car(alist) else assoc(key, cdr(alist))
```

V1 keys are integers: the key test unboxes both keys to `i64` and compares with a
typed `cmp_eq` (feeding `jmp_if_false`), because `equal?` lowers unevenly across
the managed/runtime paths — symbol keys arrive with E6d-4. Run-verified locally on
WASM (in-process) + real dotnet CLR; native/LLVM/JVM via CI.

