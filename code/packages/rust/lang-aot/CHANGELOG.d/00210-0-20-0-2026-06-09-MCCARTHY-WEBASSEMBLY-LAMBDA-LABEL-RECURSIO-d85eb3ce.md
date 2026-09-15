## 0.20.0 — 2026-06-09 — McCarthy → WebAssembly, **`LAMBDA`/`LABEL`/recursion** (LANG77 / W2)

`compile_source_to_wasm` now runs McCarthy functions: `LAMBDA` application,
multi-argument lambdas, and recursive `LABEL`. The structural pass makes the
call boundary uniform-anyref (params anyref, call args boxed, lambda returns
anyref), so `((LAMBDA (X) X) 5)` → 5, `(CDR ((LAMBDA (X Y) (CONS X Y)) 3 4))` →
4, and a recursive `LABEL` walks a list to its atom. `concretize_scalar_any_for_wasm`
skips functions with lisp params. **With this the WASM backend is McCarthy-complete
(F1–F7): cons, ATOM, EQ, COND, symbols, and lambda/label/recursion.** Twig/scalar
programs are unaffected (regression-tested).

