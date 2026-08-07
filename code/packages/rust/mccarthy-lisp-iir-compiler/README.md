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
Lisp's own VM, built on `dynval-runtime`):

```rust
use mccarthy_lisp_vm::run;
let value = run(&module).unwrap();   // → the symbol A
```

## Lowering (through L2c-3c)

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
| `((LAMBDA (p…) body) a…)` | a fresh `IIRFunction` for the lambda + a `call` to it; **captured free variables** are forwarded as leading args (L2c-3b lambda lifting) |
| `((LABEL F (LAMBDA (p…) body)) a…)` | like the lambda case, but `F` is bound while `body` is lowered, so a call `(F …)` inside `body` is a `call` back into it (**recursion**); captured free variables are forwarded as leading args (L2c-3c).  No new VM opcode: a self-call is an ordinary `call`, bounded by `MAX_CALL_DEPTH`. |
| `(LAMBDA (p…) body)` *as a value* | lift the lambda + materialise a **closure value** `(*CLOSURE* fn-name v1 … vk)` — a tagged cons whose `env` holds the captured free-variable values (empty when nothing is captured; the tag is un-forgeable since `*CLOSURE*` isn't a lexable symbol) |
| `(LABEL F (LAMBDA (p…) body))` *as a value* | a **recursive closure value** — lifted like the direct `LABEL` (so the body can recurse), then wrapped as `(*CLOSURE* label-fn . env)` (L2c-3c) |
| `(F a…)` (`F` a parameter) / `((g…) a…)` | **dynamic apply**: evaluate the head to a closure, then the `apply` opcode binds its captured `env` + the args and runs it (arity checked at run time) |

### Recursion example (L2c-2)

McCarthy's canonical `ff` — the first atom found by descending `car`s:

```lisp
((LABEL FF (LAMBDA (X)
    (COND ((ATOM X) X)
          ('T (FF (CAR X))))))
 '((A B) C))                       ; ⇒ A
```

### Capture example (L2c-3b)

The inner lambda closes over `X`, is returned, then applied later:

```lisp
(((LAMBDA (X) (LAMBDA (Y) (CONS X Y))) 'A) 'B)   ; ⇒ (A . B)
```

### Higher-order example (L2c-3a)

A lambda passed as a value, then applied via the parameter:

```lisp
((LAMBDA (F) (F 'A)) (LAMBDA (X) X))   ; ⇒ A
```

### Recursive closure example (L2c-3c)

A recursive `LABEL` passed as a value and applied — `last` walks to the
final element of a list:

```lisp
((LAMBDA (G) (G '(A B C)))
 (LABEL LAST (LAMBDA (L)
     (COND ((ATOM (CDR L)) (CAR L)) ('T (LAST (CDR L)))))))   ; ⇒ C
```

These are the conventions of the shared `dynval-runtime` value model
(tagged-`i64` symbols, cons cells, nil) — so the same IIR runs on
`mccarthy-lisp-vm` *and* feeds every IIR backend, with no new runtime
code.

## Which VM, and why its own

`vm-core`'s `Value` is scalar-only (`Int`/`Float`/`Bool`/`Str`/`Null`);
it cannot represent a symbol or a cons cell, so it cannot run
`(CAR '(A B C))` → `A`. `twig-vm` *can* — but it is the VM for the
**Twig** language, and McCarthy Lisp shouldn't ride on Twig's VM. The
thing both genuinely share is the `dynval-runtime` value model, so
McCarthy Lisp gets its **own** small VM on that foundation:
[`mccarthy-lisp-vm`](../mccarthy-lisp-vm), whose `run(&IIRModule) ->
LispyValue` executes the module against the `dynval-runtime` heap. This
crate dev-depends on it only to run the end-to-end tests.

## Closures are complete (L2c done)

As of **L2c-3c**, the full closure story is implemented: `LAMBDA` and
`LABEL` both capture free variables (precise capture / lambda lifting),
both can be used as first-class values, and a `LABEL` value is a *recursive*
closure. A bare (unquoted) symbol in value position is an *unbound variable*
unless it is a parameter of the enclosing function **or captured from an
enclosing scope** — a clean `CompileError`, never a silent mis-lowering.

The next phase (**L3**, tracked in the plan) wires `mccarthy-lisp` into
`lang-aot` so the same IIR lights up all 10 backends (AOT / VM / JIT /
WASM / JVM / CLR / BEAM / LLVM / historical archs).

## API

- `compile_source(src, module_name) -> Result<IIRModule, CompileError>`
- `compile_forms(&[LispExpr], module_name) -> Result<IIRModule, CompileError>`
