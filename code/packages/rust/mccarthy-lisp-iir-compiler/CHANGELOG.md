# Changelog — mccarthy-lisp-iir-compiler

## v0.7.1 — 2026-07-11 — DVAL01-1c: dependency renamed `lispy-runtime` → `dynval-runtime`

The shared value-model crate `lispy-runtime` is renamed to `dynval-runtime`
(spec DVAL01 §3.2). The `Cargo.toml` dependency and the `use dynval_runtime::…`
imports in the `run_e2e` integration tests move to the new name. Pure rename —
no behaviour change; the compiler still emits IIR over the same tagged-value
model that `mccarthy-lisp-vm` and every IIR backend consume.

## v0.7.0 — 2026-06-04 — LABEL capture + LABEL-as-value: recursive closures (L2c-3c)

Completes closures: `LABEL` now captures free variables and can be used as
a first-class **recursive** closure value.

```lisp
;; pass a recursive LABEL as a value, then apply it
((LAMBDA (G) (G '(A B C)))
 (LABEL LAST (LAMBDA (L)
     (COND ((ATOM (CDR L)) (CAR L)) ('T (LAST (CDR L)))))))   ; ⇒ C
```

* **`lift_label`** mirrors `lift_lambda` (precise free-variable capture,
  captured-as-leading-parameters) with two `LABEL` specifics: (1) the label
  name `F` is **bound for recursion** while the body is lowered, and (2) `F`
  is **excluded from the captured set** — it denotes the function itself
  (resolved statically), not a value to close over.  `lower_label_application`
  and `lower_label_value` now both go through it.
* **Recursive calls forward captures.**  `functions_in_scope` entries carry
  the captured names; a call `(F …)` to a labelled name lowers to
  `call label_n [captured_regs…, arg_regs…]` (via `lower_call_with_captures`),
  so a recursive body that closes over an enclosing variable threads it
  through every self-call.
* **Transitive capture.**  Because a `(F …)` call forwards `F`'s captured
  registers, those registers must be live at the call site — so
  `collect_free_symbols` now treats a called labelled function's captures as
  free symbols of the caller.  This fixes a nested-`LABEL` gap (an inner
  `LABEL` calling an outer captured `LABEL` whose captures it doesn't itself
  mention).  Each labelled function's capture set is finalised before its
  body — hence any nested caller — is lowered, so one pass handles
  arbitrarily nested `LABEL`s; IIR stays linear in source.
* **`LABEL` as a value** (a bare `(LABEL F (LAMBDA …))` in value position)
  and **a labelled name `F` used as a value** inside its own body both
  lower to a recursive closure `(*CLOSURE* label-fn . env)`, applied through
  the same `apply` opcode.  Both were errors before; the stale
  "needs closures" / "→ L2c-3b" messages are gone.
* **No `lispy-runtime` / VM change** — a self-call is an ordinary `call`,
  and a captured `env` is leading `apply` args, so a non-terminating
  recursive closure still errors cleanly (`CallDepthExceeded`).
* Internal: removed the now-unused `lower_call_to` (both label paths use the
  capture-aware `lower_call_with_captures`).
* 2 new compiler unit tests (label captures enclosing param as a leading
  param + forwards it on the recursive call; labelled name in value position
  is a recursive closure) + 2 updated (bare `LABEL` value / labelled-name
  value now lower to closures) + 4 new end-to-end tests on `mccarthy-lisp-vm`
  (LABEL body captures an enclosing variable; recursive LABEL passed as a
  value and applied; recursive LABEL value *with* capture; non-terminating
  LABEL value → call-depth guard).

## v0.6.0 — 2026-06-04 — free-variable capture for LAMBDA (L2c-3b)

Closures can now **capture** — a lambda body may reference variables from
the enclosing scope:

```lisp
(((LAMBDA (X) (LAMBDA (Y) (CONS X Y))) 'A) 'B)   ; ⇒ (A . B)
```

* **Lambda lifting with captured leading parameters.**  `lift_lambda` now
  computes the lambda's captured free variables and makes them **extra
  leading parameters** of the lifted `IIRFunction` (parameter order:
  `captured` (sorted) then the declared params).  The body lowers with
  `captured ∪ own` in scope.
* **Precise capture.**  A lambda captures exactly the free variables its
  body actually references (the body's free symbols — respecting own
  params, nested `LAMBDA`/`LABEL` binders, and `QUOTE` — intersected with
  the enclosing scope; a `BTreeSet` keeps them sorted for deterministic
  IIR).  Capturing only what's used keeps the emitted IIR **linear in the
  source**: a "capture the whole frame" strategy makes a flat fan-out of
  `k` lambdas over `m` enclosing variables emit `O(m·k)` IIR — a
  compile-time algorithmic-complexity DoS — which precise analysis avoids.
* **Two supply paths for the captured values:**
  - A **direct application** (`((LAMBDA …) a…)`) emits
    `call lambda_n [captured_regs…, arg_regs…]` — the captured registers
    (live in the caller's scope) are forwarded as leading arguments
    (`lower_call_with_captures`).
  - A **lambda used as a value** builds the closure
    `(*CLOSURE* fn-name v1 … vk)` where `env = (v1 … vk)` are the captured
    *values*; the VM's `apply` flattens `env` and prepends it to the call
    arguments on entry.  (`emit_closure` now takes the captured names.)
* The user-facing **arity check is unchanged** — against the declared
  params only; captured leading arguments are supplied implicitly.
* **No `lispy-runtime` change.**  Scoped to `LAMBDA`; `LABEL` capture +
  `LABEL`-as-value (recursive closures) are **L2c-3c** — `LABEL` keeps its
  L2c-2 own-params-only behaviour, so nothing regresses.
* 4 new compiler unit tests (inner lambda captures enclosing param as a
  leading param; direct inner application forwards the captured register;
  top-level lambda captures nothing; **capture is precise** — an inner
  lambda that doesn't reference the outer var captures nothing) + 5 new
  end-to-end tests on
  `mccarthy-lisp-vm` (curry / capture-then-apply-later, direct-application
  capture, transitive 2-level capture, captured closure passed to a
  higher-order function, shadowing param is not captured).

## v0.5.0 — 2026-06-04 — closures as values + dynamic apply (L2c-3a)

First half of closures (L2c-3 is split into 3a/3b).  A `LAMBDA` is now a
**first-class value**, and a value can be **applied**.

* **Lambda as a value.**  A `LAMBDA` in value position (an argument, a
  returned form, or standing alone) lowers to a **closure value** — the
  tagged cons `(*CLOSURE* fn-name)` built from existing `cons`es over the
  shared `lispy-runtime` model (the captured environment is empty in 3a).
  The tag `*CLOSURE*` is **un-forgeable from source**: a McCarthy symbol is
  `[A-Z][A-Z0-9-]*`, so the lexer can never produce `*CLOSURE*` and no
  `QUOTE` can fabricate a value the VM will accept as a closure.
* **Dynamic apply.**  A call whose head is a **parameter** (`(F a…)`) or a
  **nested application** (`((g…) a…)`) lowers to the new VM `apply` opcode:
  the head is evaluated to a closure value and dispatched at run time
  (arity is checked by the VM, since the callee isn't known statically).
  Higher-order functions now work: `((LAMBDA (F) (F 'A)) (LAMBDA (X) X))`
  → `A`; a returned lambda can be applied: `(((LAMBDA (X) (LAMBDA (Y) Y))
  'Z) 'W)` → `W`.
* **No `lispy-runtime` change** (closure conversion + encode-in-cons), so
  the per-PR Miri obligation still does not apply.  Each lambda lifts to a
  top-level `IIRFunction` exactly as in L2c-1 (now via a shared
  `lift_lambda` helper used by both direct application and lambda-value);
  the only new IIR is the closure `cons`es and the `apply` op.
* **Still deferred to L2c-3b:** free-variable capture — a lambda body still
  sees only its own parameters, so a reference to an enclosing binding is
  an unbound-variable error.  `LABEL`-as-value (a *recursive* closure) is
  also 3b; a bare `LABEL` value now reports "→ L2c-3b".  Applying a
  non-function (an integer / empty list) is a compile error; applying a
  runtime non-closure is the VM's clean `NotAClosure`.
* Dispatch precedence in `lower_application` is now: labelled name
  (recursion) → parameter (dynamic apply) → primitive table → `LAMBDA`
  value → unknown.  A parameter therefore lexically shadows a primitive of
  the same spelling (correct scoping).
* 7 new compiler unit tests (bare-lambda→closure shape, un-forgeable tag,
  apply-a-parameter emits `apply` on the param register, apply-a-nested-
  application, cannot-apply-an-integer, label-value→3b) + 6 new end-to-end
  tests on `mccarthy-lisp-vm` (higher-order identity, higher-order
  primitive closure, returned-then-applied, bare-lambda-is-a-closure-pair,
  apply-non-closure → clean error, Ω combinator → call-depth guard).

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
