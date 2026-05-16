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
integration, and deferred-form rejection.

## Related crates

- [`semantic-ir`](../semantic-ir/) — the IR itself
- [`semantic-ir-to-typescript`](../semantic-ir-to-typescript/) —
  first backend
