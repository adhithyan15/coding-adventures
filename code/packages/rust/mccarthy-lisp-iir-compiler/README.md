# mccarthy-lisp-iir-compiler

Lowers a **McCarthy 1960 Lisp** (Lisp 1.0) AST into an **IIRModule** —
the architecture-independent IR that every backend in the chain
consumes (the McCarthy VM, the JIT, wasm/jvm/clr/beam, the
historical-arch encoders).

This is **L2a** of the McCarthy Lisp implementation — see
[`MCCARTHY-LISP-PLAN.md`](../../../specs/MCCARTHY-LISP-PLAN.md).

## What it does

A McCarthy program is a sequence of top-level S-expressions. They lower
into a single IIR function `main` whose value is the value of the last
form (an empty program returns `nil`).

```rust
use mccarthy_lisp_iir_compiler::compile_source;

let module = compile_source("(CAR '(A B C))", "demo").unwrap();
assert_eq!(module.entry_point.as_deref(), Some("main"));
assert!(module.validate().is_empty());
```

The emitted module runs end-to-end on **`mccarthy-lisp-vm`** (McCarthy
Lisp's own VM, built on `lispy-runtime`):

```rust
use mccarthy_lisp_vm::run;
let value = run(&module).unwrap();   // → the symbol A
```

## Lowering (through L2c-3a)

| Form               | IIR                                                     |
|--------------------|---------------------------------------------------------|
| `42`               | `const v, Int(42) : i64`                                |
| `()`               | `const v, Int(0) : ref<LispyPair>`  (the nil sentinel)  |
| `'X` / `(QUOTE X)` | materialise `X` as data (symbol → `const Var`, list → cons chain) |
| `(CONS A B)`       | `call_builtin "cons" [A, B]`                            |
| `(CAR X)`          | `call_builtin "car" [X]`                                |
| `(CDR X)`          | `call_builtin "cdr" [X]`                                |
| `(ATOM X)`         | `(not (pair? X))` — two `call_builtin`s                 |
| `(EQ A B)`         | `call_builtin "equal?" [A, B]` (identity on atoms)      |
| `(COND (p e) …)`   | chained `jmp_if_false` + `label`s; each clause's value funnels into one register via `mov`; no match → `nil` |
| `((LAMBDA (p…) body) a…)` | a fresh `IIRFunction` for the lambda + a `call` to it with the lowered args (params bound by name; no free-variable capture) |
| `((LABEL F (LAMBDA (p…) body)) a…)` | like the lambda case, but `F` is bound to the new function while `body` is lowered — so a call `(F …)` inside `body` becomes a `call` back into it, i.e. **recursion**.  No new VM opcode: a self-call is an ordinary `call`, bounded by `MAX_CALL_DEPTH`. |
| `(LAMBDA (p…) body)` *as a value* | lift the lambda + materialise a **closure value** `(*CLOSURE* fn-name)` — a tagged cons (empty env in L2c-3a; the tag is un-forgeable since `*CLOSURE*` isn't a lexable symbol) |
| `(F a…)` (`F` a parameter) / `((g…) a…)` | **dynamic apply**: evaluate the head to a closure, then the `apply` opcode runs it (arity checked at run time) |

### Recursion example (L2c-2)

McCarthy's canonical `ff` — the first atom found by descending `car`s:

```lisp
((LABEL FF (LAMBDA (X)
    (COND ((ATOM X) X)
          ('T (FF (CAR X))))))
 '((A B) C))                       ; ⇒ A
```

### Higher-order example (L2c-3a)

A lambda passed as a value, then applied via the parameter:

```lisp
((LAMBDA (F) (F 'A)) (LAMBDA (X) X))   ; ⇒ A
```

These are the conventions of the shared `lispy-runtime` value model
(tagged-`i64` symbols, cons cells, nil) — so the same IIR runs on
`mccarthy-lisp-vm` *and* feeds every IIR backend, with no new runtime
code.

## Which VM, and why its own

`vm-core`'s `Value` is scalar-only (`Int`/`Float`/`Bool`/`Str`/`Null`);
it cannot represent a symbol or a cons cell, so it cannot run
`(CAR '(A B C))` → `A`. `twig-vm` *can* — but it is the VM for the
**Twig** language, and McCarthy Lisp shouldn't ride on Twig's VM. The
thing both genuinely share is the `lispy-runtime` value model, so
McCarthy Lisp gets its **own** small VM on that foundation:
[`mccarthy-lisp-vm`](../mccarthy-lisp-vm), whose `run(&IIRModule) ->
LispyValue` executes the module against the `lispy-runtime` heap. This
crate dev-depends on it only to run the end-to-end tests.

## Not yet (later phases)

- **Free-variable capture (L2c-3b)** — a lambda body still sees only its
  own parameters; a reference to an enclosing binding is an
  unbound-variable error until capture threads it through the closure's
  environment (the `env` slot, empty in L2c-3a). **`LABEL` as a value** (a
  recursive closure) also lands in L2c-3b.

A bare (unquoted) symbol in value position is an *unbound variable* unless
it is a parameter of the enclosing lambda; a labelled function name used
in value position is reported as needing L2c-3b. Either way it is a
`CompileError` rather than silently mis-lowered.

## API

- `compile_source(src, module_name) -> Result<IIRModule, CompileError>`
- `compile_forms(&[LispExpr], module_name) -> Result<IIRModule, CompileError>`
