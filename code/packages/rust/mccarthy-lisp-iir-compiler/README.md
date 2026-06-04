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

## Lowering (L2a scope)

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

- **L2b** — `COND` (chained `jmp_if_false` + labels).
- **L2c** — `LAMBDA` / `LABEL` / user-defined function application.

A bare (unquoted) symbol in value position is an *unbound variable* in
L2a (there are no bindings until LAMBDA/LABEL) and is reported as a
`CompileError`.

## API

- `compile_source(src, module_name) -> Result<IIRModule, CompileError>`
- `compile_forms(&[LispExpr], module_name) -> Result<IIRModule, CompileError>`
