# mccarthy-lisp-iir-compiler

Lowers a **McCarthy 1960 Lisp** (Lisp 1.0) AST into an **IIRModule** —
the architecture-independent IR that every backend in the chain
consumes (twig-vm, vm-core, the JIT, wasm/jvm/clr/beam, the
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

The emitted module runs end-to-end on **`twig-vm`**:

```rust
use twig_vm::dispatch::run;
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

These are the exact opcode + builtin conventions `twig-ir-compiler`
emits and `twig-vm` / the IIR backends already execute, so McCarthy
Lisp — Twig's untyped cousin — reuses the whole `lispy-runtime` value
model (tagged-`i64` symbols, cons cells, nil) for free.

## Why twig-vm, not vm-core

`vm-core`'s `Value` is scalar-only (`Int`/`Float`/`Bool`/`Str`/`Null`);
it cannot represent a symbol or a cons cell, so it cannot run
`(CAR '(A B C))` → `A`. `twig-vm`'s `dispatch::run(&IIRModule) ->
LispyValue` executes the module against the cons-capable
`lispy-runtime` heap. No `twig-vm` source is modified — this crate only
dev-depends on it to run its end-to-end tests.

## Not yet (later phases)

- **L2b** — `COND` (chained `jmp_if_false` + labels).
- **L2c** — `LAMBDA` / `LABEL` / user-defined function application.

A bare (unquoted) symbol in value position is an *unbound variable* in
L2a (there are no bindings until LAMBDA/LABEL) and is reported as a
`CompileError`.

## API

- `compile_source(src, module_name) -> Result<IIRModule, CompileError>`
- `compile_forms(&[LispExpr], module_name) -> Result<IIRModule, CompileError>`
