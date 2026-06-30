# SIR19 — JavaScript → Semantic IR

## Status

Third frontend for the narrow-waist Semantic IR (after Twig in SIR11
and Python in SIR17).  Consumes the existing
[`javascript-parser`](../packages/rust/javascript-parser/) crate and
produces a `semantic_ir::Module`.  Implemented as the Rust crate
`javascript-to-semantic-ir`.

## Pipeline

```text
JavaScript source
   │
   ▼  javascript_parser::parse_javascript(source, "es2020")
parser::grammar_parser::GrammarASTNode (generic CST)
   │
   ▼  javascript_to_semantic_ir::compile_source
semantic_ir::Module                          (per SIR10 + SIR16)
```

## Public API

```rust
pub fn compile(
    tree:        &GrammarASTNode,
    module_name: &str,
) -> Result<semantic_ir::Module, JsLowerError>;

pub fn compile_source(
    source:      &str,
    module_name: &str,
) -> Result<semantic_ir::Module, JsLowerError>;  // parse + lower

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsLowerError {
    pub message: String,
    pub line:    usize,
    pub column:  usize,
}
```

JS source is parsed with the `"es2020"` grammar version by default
(arrow functions, template literals, `let`/`const`, spread are all in
scope; the spread parts of the language we then reject at lowering).

## Subset coverage (v0 MVP)

| JavaScript source                            | SIR lowering                                            |
|----------------------------------------------|---------------------------------------------------------|
| `42`, `-7`                                   | `IntLit { value }`                                      |
| `3.14`                                       | `FloatLit { value }`                                    |
| `true`, `false`                              | `BoolLit { value }`                                     |
| `null`, `undefined`                          | `NilLit`                                                |
| `"hello"`, `'world'`                         | `StrLit { value }`                                      |
| ``` `template ${x} str` ```                  | desugar to `+`-concat: `"template " + x + " str"`        |
| `name` (reference)                           | `VarRef { name, scope }`                                |
| `let x = 1;` / `const x = 1;` / `var x = 1;` | `LetBinding`                                            |
| `x = 1;` (re-assignment)                     | `Assign`                                                |
| `x + y` etc.                                 | `BuiltinCall("+", ...)` (and friends)                   |
| `x === y`, `x !== y`                         | `BuiltinCall("=", ...)` / `BuiltinCall("!=", ...)`      |
| `x == y`, `x != y`                           | same as above (loose / strict normalized to strict)     |
| `x < y`, `x > y`, `x <= y`, `x >= y`         | `BuiltinCall("<", ...)` etc.                            |
| `x && y`, `x \|\| y`                         | `LogicalAnd` / `LogicalOr`                              |
| `!x`                                         | `BuiltinCall("not", [x])`                               |
| `-x`                                         | `BuiltinCall("neg", [x])`                               |
| `if (c) { ... } else { ... }`                | `If`                                                    |
| `if (c) {} else if (c2) {} else {}`          | nested `If`                                             |
| `while (c) { body }`                         | `While`                                                 |
| `for (let i = 0; i < n; i++) { body }`       | `ForRange { var: i, start: 0, stop: n, step: 1, body }` |
| `for (let i = 0; i < n; i += step) { body }` | `ForRange` with step                                    |
| `for (const x of xs) { body }`               | `ForEach`                                               |
| `function f(a, b) { body }`                  | `Function`                                              |
| `(a, b) => body`                             | `MakeClosure` (synthesised `Function`)                  |
| `(a) => expr`                                | same                                                    |
| `return expr;`                               | `Block.value = expr` (see "Return" below)               |
| `f(arg1, arg2)`                              | `DirectCall` / `IndirectCall`                           |
| `console.log(x)`                             | `BuiltinCall("print", [x])`                             |
| `xs.length`                                  | `SeqLen { seq: xs }`                                    |
| `[1, 2, 3]`                                  | `SeqLit { items }`                                      |
| `xs[i]`                                      | `SeqIndex { seq: xs, index: i }`                        |
| `xs[i] = v;`                                 | `SeqSet`                                                |
| `{ "a": 1, "b": 2 }` / `{ a: 1, b: 2 }`      | `MapLit { entries }`                                    |
| `d[k]` / `d.k`                               | `MapGet { map: d, key: k }`                             |
| `d[k] = v;` / `d.k = v;`                     | `MapSet`                                                |

## let / const / var

All three become a binding statement.  **Implementation note (M2):** the
frontend emits `Stmt::LetStarBinding` (sequential `let*`), **not**
`Stmt::LetBinding`.  The coverage table above writes "LetBinding"
generically, but the SIR validator treats a run of consecutive
`LetBinding`s as a *parallel* group whose right-hand sides may not see
one another, whereas JS `let`/`const`/`var` are sequentially scoped — a
plain `let x = 1; const y = x + 1;` must validate.  `let*`'s sequential
semantics match JS exactly, so it is the correct lowering for ordered
top-level declarations.

The frontend doesn't preserve the
`let`/`const`/`var` distinction in v0 — the IR doesn't model immutability
constraints, and
JS's runtime treats `const` as advisory anyway.  A subsequent
re-assignment to a `const`-declared name in source is **silently
accepted** by the frontend (no error); the round-tripped Python or
target output may produce a different runtime behaviour than the JS
input.  Future versions may track `Const` as a Feature.

## Equality normalisation

JS distinguishes `==` (loose) and `===` (strict).  The IR has only
`=` (strict-shaped — types must match).  The frontend normalises both
JS forms to the strict comparison, *changing semantics* for
`null == undefined` (true in JS, false in SIR/strict).  In v0 we
accept this loss; programs relying on loose-equality coercion are
explicitly out of MVP scope.

A future spec revision could add `Feature::LooseEquality` and a
distinct `BuiltinCall("==", ...)` if needed.

## `undefined` vs `null`

Both lower to `NilLit`.  The distinction is JS-specific and not
preserved.  Programs that check `=== undefined` versus `=== null`
will exhibit subtle bugs on round-trip — these are out of MVP
scope.

## Return statement

JavaScript permits early `return` mid-function.  Same as SIR17 (Python),
the IR doesn't have an early-return node — every function body is a
`Block` whose `value` is the return.  v0 frontend:

- Function with single trailing `return expr` → `body.value = expr`.
- Function with no return / trailing `return;` → `body.value = NilLit`.
- Function with early `return` mid-body → **rejected** with clear error.

Future: lift to a control-flow shape.

## Arrow functions vs `function` declarations

Both lower to the same shape.  Arrow function with non-block body
(`x => x + 1`) becomes a `MakeClosure` whose body is a `Block` with no
statements and `value = x + 1`.  Block body (`(x) => { stmts; return r; }`)
becomes a `MakeClosure` whose body is a `Block` with the statements
and `value = r`.

`this`-binding differences between arrow functions and `function`
declarations are not preserved.  v0 doesn't support `this` at all —
no class support means no `this` cases worth modelling.

## `var` hoisting

JavaScript hoists `var` declarations to function scope.  In v0, the
frontend emits `var` declarations as `LetBinding`s at the point they
appear in source — hoisting is NOT modelled.  Programs that depend on
hoisting (reference a `var` before its source-position declaration)
will fail SIR validation with an unresolved-name error.  Future:
implement a hoisting pre-pass.

## Top-level

JS source is a sequence of top-level statements.  The frontend emits a
synthetic `main` function containing all of them, matching SIR17's
approach.  Top-level `function` declarations become user-visible
`Function`s in the module (`function f() {}` at module scope is the JS
analog of Python `def f(): ...` at module scope).

## Closures

Same model as SIR11 / SIR17.  Arrow functions and inner `function`s
have their free variables computed; those become `Capture`s on the
synthesised `Function`.

## Manifest computation

Same trigger map as SIR17 (Python).

## Error model

```rust
JsLowerError {
    message: String,
    line:    usize,
    column:  usize,
}
```

Errors:

- Unresolved name reference
- Early `return` mid-function
- Reassignment to a `const` — actually NOT an error in v0; permitted.
- Unsupported syntax: `class`, `try`/`catch`, `throw`, `async`/`await`,
  generators, destructuring, spread, default parameters, rest parameters,
  `with`, `eval`, `new`, `this` outside permitted contexts.
- `import` / `export` (multi-module is out of scope)

## Tests

`cargo test -p javascript-to-semantic-ir`:

- Per-rule positive lowering test
- Negative tests for rejected syntax
- Golden tests for canonical programs (factorial, fibonacci, list-sum,
  dict-access, closure-adder)
- End-to-end JS → SIR → Python via `semantic-ir-to-python` extended in
  SIR20.  Tests that run on machines with `python3` installed compare
  stdout to expected.

Coverage target ≥ 90%.

## Out of scope (deferred)

- Classes / prototypes / `new` / `this`
- Exceptions (`try`/`catch`/`throw`)
- Generators / `yield`
- `async function` / `await`
- Destructuring (`const { a, b } = obj;`)
- Spread / rest (`...args`)
- Default parameters
- Tagged template literals
- ES module `import` / `export`
- `eval` / `Function()` constructor
- Regular expressions
- Prototype methods on built-in types (Array, String, etc.)
- `for-in` loops over object keys (use map iteration instead — also
  out of v0 scope)
