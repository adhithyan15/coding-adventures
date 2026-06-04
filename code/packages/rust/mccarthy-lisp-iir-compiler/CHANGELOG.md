# Changelog — mccarthy-lisp-iir-compiler

## v0.2.0 — 2026-06-04 — COND (L2b)

* Lowers `(COND (p1 e1) … (pn en))` to a chain of `jmp_if_false` +
  `label`s, with each clause's value funnelled into one `result`
  register via `mov`. The value is the `ei` of the first true `pi`; if no
  clause matches, the result is `nil` (the total extension of McCarthy's
  otherwise-undefined `cond`).
* Each clause must be a 2-element list `(predicate expression)` —
  malformed clauses are a `CompileError`. The catch-all clause uses a
  truthy predicate such as `('T …)` (a bare `T` is still an unbound
  variable until bindings arrive in L2c).
* New emitters: `mov` / `label` / `jmp` / `jmp_if_false`, plus a
  fresh-label generator. The emitted IIR passes `IIRModule::validate`
  (all branch targets are defined).
* 3 new unit tests (branch/funnel op shape, clause-arity errors, empty
  `COND`) + 7 new end-to-end tests on `mccarthy-lisp-vm` (first-true
  clause, fall-through, `EQ` predicates, no-match→nil, COND value feeding
  an enclosing form, nested COND, COND returning a cons).

## v0.1.0 — 2026-06-03 — initial release (L2a)

First IIR lowering for McCarthy 1960 Lisp — the L2a slice of the
McCarthy Lisp plan.

* `compile_source(src, name) -> Result<IIRModule, CompileError>` and
  `compile_forms(&[LispExpr], name)` — lower a top-level form sequence
  into a single `main` function returning the value of the last form
  (empty program → `nil`).
* Supported forms: integer literals, `()`/nil, `QUOTE` (symbols +
  integers + nested lists → cons chains), and the primitives `CONS`,
  `CAR`, `CDR`, `ATOM` (lowered to `not pair?`), and `EQ` (lowered to
  `equal?`, which is identity on atoms — the numeric `=` builtin rejects
  symbols).
* Emits the shared `lispy-runtime` IIR conventions
  (`const`/`call_builtin "cons"`/`"car"`/`"cdr"`/`"pair?"`/`"not"`/`"equal?"`,
  `const Var(name)` for symbols, `const 0 : ref<LispyPair>` for nil), so
  the module runs end-to-end on `mccarthy-lisp-vm` and feeds every IIR
  backend.
* `CompileError` reports unsupported forms (`COND`→L2b, `LAMBDA`/`LABEL`→L2c),
  unbound bare symbols, wrong arity, and unknown primitives — with
  actionable messages.

### Tests

* 15 unit tests pin the emitted IIR shape (op sequences, `may_alloc` on
  `cons`, symbol-as-`Var`, nil-as-`ref<LispyPair>`-zero, error paths).
* 16 end-to-end tests run compiled programs on `mccarthy-lisp-vm` and
  assert the resulting `LispyValue`: `(CAR '(A B C))` → `A`,
  `(CDR '(A B C))` → `(B C)`, `(CONS 'A 'B)` → `(A . B)`, `(ATOM …)`,
  `(EQ …)`, nested quotes, dotted pairs, and multi-form sequencing.

### Notes

* **Execution VM:** runs on `mccarthy-lisp-vm` — McCarthy Lisp's own VM,
  built on `lispy-runtime`.  Not `vm-core` (scalar-only: it cannot
  represent symbols / cons cells) and deliberately not `twig-vm` (that
  VM is Twig-specific; the two languages share only the `lispy-runtime`
  value model).  No `lispy-runtime` / `lang-runtime-core` source is
  modified, so the per-PR Miri obligation does not apply.
