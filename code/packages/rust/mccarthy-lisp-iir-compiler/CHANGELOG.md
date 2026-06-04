# Changelog — mccarthy-lisp-iir-compiler

## v0.4.0 — 2026-06-04 — LABEL: named / recursive functions (L2c-2)

* Lowers a direct application of a *named* lambda
  `((LABEL F (LAMBDA (p1 … pn) body)) a1 … an)`.  It compiles exactly like
  a direct `LAMBDA` application (one fresh top-level `IIRFunction`, gensym
  `label_<n>`, a `call` from the caller) **plus** one thing: the name `F`
  is bound — in a new *function scope* — to that function *before* the
  body is lowered.  A call `(F …)` whose head is a function-scope name
  lowers to a `call` to that function, so a body that calls `F` **recurses**.
* **No new VM opcode.**  A self-call is an ordinary `call`; the VM already
  resolves the callee by name and runs it in a fresh frame, bounded by
  `MAX_CALL_DEPTH` + the shared instruction budget — so a non-terminating
  `LABEL` errors cleanly (`CallDepthExceeded`) instead of overflowing the
  native stack.  `mccarthy-lisp-vm` is unchanged except for docs and tests.
* The function scope is **lexically scoped**: `F`'s binding is saved/
  restored around the body, so `F` is invisible outside the `LABEL` (a
  sibling reference is unbound), and an inner binding shadows an outer one
  (and shadows a primitive of the same spelling — the correct rule).
* **Still no closures (L2c-3):** a labelled name used in *value* position
  (passed or returned, not called) is rejected with a clear
  closures-needed message; a bare, unapplied `LABEL` is rejected the same
  way a bare `LAMBDA` is.  Recursive-call arity, malformed `LABEL` shape
  (wrong element count, non-symbol name, non-`LAMBDA` body) are
  `CompileError`s; the emitted multi-function module passes
  `IIRModule::validate`.
* Internal refactor: direct `LAMBDA` and `LABEL` application now share a
  `lower_call_to` emitter for the argument-lowering + `call` tail.
* 11 new compiler unit tests (function+call shape, recursive self-call
  lowering, value-position rejection, recursive-call/application arity,
  malformed-`LABEL` paths, lexical scope) + 5 new end-to-end tests on
  `mccarthy-lisp-vm` (identity, McCarthy's canonical `ff` first-atom,
  `last` over a cdr-spine + its single-element base case, and a
  non-terminating recursion hitting the call-depth guard).

## v0.3.0 — 2026-06-04 — direct LAMBDA application (L2c-1)

* Lowers direct lambda application `((LAMBDA (p1 … pn) body) a1 … an)`:
  the lambda becomes a fresh top-level `IIRFunction` (gensym
  `lambda_<n>`) with parameters `p1…pn`, and the application emits a
  `call` to it with the lowered argument registers.
* The compiler now produces **multiple functions** (the lambdas + `main`)
  and tracks the **parameter scope** of the function being lowered. A
  bare symbol resolves to a register read **only** if it is a parameter
  of the enclosing lambda; the VM binds each parameter to a register
  named after it.
* **No closures yet (deferred):** a lambda body may reference only its
  own parameters — a free variable is an unbound-variable error, and an
  unapplied `LAMBDA` (lambda-as-value) is rejected with a clear message.
  `LABEL` (named/recursive functions) is deferred to **L2c-2**.
* Validation: malformed lambdas (wrong shape, non-symbol or duplicate
  parameters, arity mismatch) are `CompileError`s; the emitted multi-
  function module passes `IIRModule::validate`.
* 7 new unit tests (function+call shape, param-in-scope, free-variable
  unbound, arity, duplicate param, bare-lambda rejected, LABEL deferred)
  + 8 new end-to-end tests on `mccarthy-lisp-vm` (identity on
  symbol/int, `CAR` of arg, two-param `CONS`, param used twice, body
  using `COND`, argument that is itself a lambda application, lambda
  result feeding a primitive).

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
