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

## Control flow (M3)

**Implementation note (M3).**  `if`/`else`, `while`, the C-style `for`,
and `for … of` are implemented as of crate `0.3.0`, along with bare
`{ … }` blocks.  Details and the two deliberate divergences from a naive
reading of the table above:

- **`if` is an expression.**  The IR has no statement-level `if`; an `if`
  *statement* lowers to `Stmt::ExprStmt` wrapping an `Expr::If`.  A
  missing `else` gets a synthetic empty nil-valued `Block`.  Else-if
  chains nest a further `Expr::If` in the outer `else_branch`'s tail
  value.  `Expr::If` is **not** gated by any `Feature` (the validator
  observes none), so a conditional adds nothing to the manifest.
- **C-style `for` is accepted only for the canonical counting shape.**
  Init `let i = <start>`, cond `i < <stop>` or `i <= <stop>` on the same
  `i`, update `i = i + <step>` / `i += <step>` / `i++`.  `i <= <stop>` is
  rewritten to the half-open `i < <stop> + 1` (the IR `ForRange` is
  half-open, `stop` exclusive).  **Non-canonical** loops — a different
  loop variable across clauses, a decrement (`i--`), a multiplicative
  step (`i = i * 2`), a missing clause, or a multi-variable init — are
  rejected with a positioned `JsLowerError` (deferred) rather than
  mis-lowered.  This is a tightening of the bare table rows above: only
  the counting form is in scope for v0.
- **`for … of`** supports the single-identifier binding only;
  destructuring is deferred.
- **Scoping.**  The loop variable is bound into the loop body scope only
  (visible inside, unresolved after); names bound inside any control-flow
  body or bare block are block-scoped and do not leak outward — matching
  the SIR validator's `LocalEnv` mark/rewind around each `Block` and each
  loop body.
- **Recursion bound.**  Statement-block nesting is capped at
  `MAX_STMT_DEPTH = 256` (the twin of the M2 `MAX_EXPR_DEPTH`), so deeply
  nested control flow yields a positioned error, not a stack overflow.

Still deferred after M3: `switch`, `try`/`catch`, `do … while`, labeled
statements, and `break`/`continue` (the IR has no early-exit node).

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

**Implementation note (M4).**  Implemented as of crate `0.4.0`.  One
refinement over a literal reading of "single trailing `return`": a
`return` is in *tail position* — and therefore accepted — when it is the
body's last statement **or**, recursively, the last statement of a branch
of a tail-position `if`.  A tail `if (c) { return a; } else { return b; }`
folds into an `Expr::If` whose branch values are the returns.  This admits
the natural guard-style recursion shape (factorial / fibonacci goldens)
without an early-return node, while a `return` that is followed by more
statements — the genuinely *early* case — is still rejected with a
positioned `JsLowerError` ("early return not supported in v0").

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

**Implementation note (M4).**  Implemented as of crate `0.4.0`.  Free
variables are discovered **on resolve**: a closure body is lowered inside
a fresh scope frame, and each name that resolves to an *enclosing* frame's
local/param/capture (and is not a param/local of the closure, nor a module
function/global/builtin) is recorded as a `Capture` (resolved as
`Scope::Capture` inside the body) with a matching `CaptureValue` on the
`MakeClosure` (resolved in the enclosing scope).  Captures thread
**transitively** through nested closures.  Arrow functions get gensym'd
`__lambda_<N>` names; nested `function` declarations keep their source
name (lifted to top level) and are bound locally to a `MakeClosure`.

## Manifest computation

Same trigger map as SIR17 (Python).

**Implementation note (M4).**  The SIR validator has no node it can use to
*observe* `Feature::MutualRecursion`, so the frontend declares it itself —
but **only** when a genuine cycle of length ≥ 2 exists in the static call
graph (detected over the lowered `DirectCall`s).  Pure self-recursion is
not mutual recursion.  This keeps the declared manifest honest and avoids a
spurious "declared but unused" validator warning.  `Feature::Closures` is
declared for any arrow / nested function / `IndirectCall`;
`Feature::DynamicTyping` for any untyped param (every JS param in v0).

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
