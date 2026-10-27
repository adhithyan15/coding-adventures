# twig-to-semantic-ir

The first frontend for the narrow-waist Semantic IR — lowers
[twig-parser](../twig-parser/)'s typed AST into a
[semantic-ir](../semantic-ir/) module.  Implements
[SIR11](../../../specs/SIR11-twig-to-semantic-ir.md).

## Pipeline

```text
Twig source
   │
   ▼  twig_parser::parse
twig_parser::Program (typed AST)
   │
   ▼  twig_to_semantic_ir::compile
semantic_ir::Module (SIR v0)
```

## Public API

```rust
use twig_to_semantic_ir::{compile, compile_source, TwigLowerError};

// Two entry points:
let module = compile(&parsed_program, "demo")?;
let module = compile_source("(+ 1 2)", "demo")?;  // parse + lower
```

## Coverage

Implements the entire TW00 surface:

- Literals (int, bool, nil, symbol, string)
- VarRef with explicit scope tags (`Local` / `Param` / `Capture` /
  `Global` / `Builtin`)
- `if`, `let` (parallel), `let*` (sequential), `begin`
- `lambda` — synthesised into a fresh top-level function with
  computed captures + `MakeClosure` at the source position
- `define` — value form → `Global` + ExprStmt in `_init`; function
  form → top-level `Function`
- Apply-site dispatch picks `DirectCall` / `IndirectCall` /
  `BuiltinCall`

Deferred (errors at lowering time): `match`, records, unions, type
aliases.

## Name resolution

Twig is lexically scoped, like Scheme.  Every name resolves in the same
order (TW00 "Name resolution"), whether it is read as a value or called.
The first row that binds the name wins:

| Row | Bound by                                                   | `name` reads as         | `(name args…)` lowers to   |
|-----|------------------------------------------------------------|-------------------------|----------------------------|
| 1   | parameter, `let` / `let*`, lambda parameter, capture        | `VarRef` Local/Param/Capture | `IndirectCall` on it   |
| 2   | top-level function `define`                                | `VarRef` Global         | `DirectCall name`          |
| 3   | top-level value `define`                                   | `VarRef` Global         | `IndirectCall` on it       |
| 4   | builtin (`+`, `cons`, `car`, `print`, …)                   | `VarRef` Builtin        | `BuiltinCall name`         |

So redefining a builtin is allowed, and the definition wins everywhere:

```scheme
(define (car p) 42)
(car (cons 1 2))                 ; DirectCall car → 42, not the builtin's 1
(define (f car) (car 1))         ; IndirectCall on the parameter
(let ((car (lambda (p) 9)))      ; IndirectCall on the let binding → 9
  (car 1))
(cdr (cons 1 2))                 ; cdr is not shadowed: BuiltinCall cdr → 2
```

A define whose expression is a bare `lambda`, such as
`(define g (lambda (x) x))`, counts as a function define (row 2).
Special forms (`if`, `let`, `let*`, `lambda`, `begin`, `quote`) are
syntax, parsed before resolution runs, so a definition cannot shadow
them.  Before 0.2.1 call position checked row 4 first (issue #18456).

## Closure model

Each `(lambda (x) ...)` becomes:

1. A fresh `__lambda_<N>` top-level `Function` with computed captures
   (free variables not bound at the lambda site).
2. A `MakeClosure { fn_name = "__lambda_N", captures = [...] }` at
   the original lambda position.

Inside the synthesised function, captured names resolve as
`VarRef { scope: Capture }`.  Names that aren't captured (globals,
top-level function names, builtins) resolve directly without
appearing in the capture list.

## Tests

`cargo test -p twig-to-semantic-ir`

Covers each lowering rule, scope resolution corner cases (parallel
let vs sequential let*), closure capture computation, validator
integration, and deferred-form rejection.  `tests/name_resolution.rs`
pins the call node chosen for each resolution row, including every
builtin shadowed by a function, a value, a parameter, a `let` binding,
a lambda parameter and a capture.  `semantic-ir-to-javascript`'s
`tests/twig_name_resolution.rs` runs those programs under `node`.

## Related crates

- [`semantic-ir`](../semantic-ir/) — the IR itself
- [`semantic-ir-to-typescript`](../semantic-ir-to-typescript/) —
  first backend
