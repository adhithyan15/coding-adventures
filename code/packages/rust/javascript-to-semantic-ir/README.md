# javascript-to-semantic-ir

JavaScript → narrow-waist [Semantic IR](../semantic-ir/) frontend (SIR19).

This crate is the JavaScript entry point into the Semantic IR (SIR)
pipeline, joining the existing Twig (SIR11), Python (SIR17), and Ruby
frontends. It consumes the **generic concrete syntax tree**
(`parser::grammar_parser::GrammarASTNode`) emitted by the
[`javascript-parser`](../javascript-parser/) crate and produces a
`semantic_ir::Module` that any SIR backend (`semantic-ir-to-rust`,
`-typescript`, `-go`, `-python`) can consume.

```text
JavaScript source
   │
   ▼  javascript_parser::parse_javascript(source, "es2020")
parser::grammar_parser::GrammarASTNode   (generic CST)
   │
   ▼  javascript_to_semantic_ir::compile_source     ← THIS CRATE
semantic_ir::Module
   │
   ▼  semantic-ir-to-{rust, typescript, go, python}
target source
```

We lower from the *generic* `GrammarASTNode`, **not** from the parser's
typed-AST bridge (`javascript-parser/src/bridge.rs`). That is the
contract SIR19 sets, matching how the Ruby and Twig frontends work.

## Milestone status — M5 (collections: arrays & objects)

This is the fifth slice of SIR19. It implements literals (M1),
variables/operators (M2), control flow (M3), and functions/closures (M4)
**plus** collections: array literals, object literals, member / dot /
subscript access (reads), and indexed / property assignment (writes). The
supported subset (M1 + M2 + M3 + M4 + M5):

| JavaScript source           | SIR lowering                                  |
|-----------------------------|-----------------------------------------------|
| `42`, `0`, `-`-free ints    | `IntLit { value }`                            |
| `3.25`, `1e3`, `1.5e-3`     | `FloatLit { value }`                          |
| `-5`, `-3.25`               | constant-folded `IntLit(-5)` / `FloatLit(-3.25)` |
| `true`, `false`             | `BoolLit { value }`                           |
| `null`                      | `NilLit`                                       |
| `undefined`                 | `NilLit` (distinction from `null` is lost)    |
| `"hi"`, `'hi'`              | `StrLit { value }`                            |
| `name` (declared reference) | `VarRef { name, scope: Local }`               |
| `let`/`const`/`var x = e;`  | `LetStarBinding` (see "Bindings" below)       |
| `x = e;` (re-assignment)    | `Assign { scope: Local }`                     |
| `a + b`, `a - b`, `a * b`, `a / b`, `a % b` | `BuiltinCall("+"/"-"/"*"/"/"/"%")` |
| `a < b`, `a > b`, `a <= b`, `a >= b` | `BuiltinCall("<"/">"/"<="/">=")`     |
| `a == b`, `a === b`         | `BuiltinCall("=")` (strict-normalised)        |
| `a != b`, `a !== b`         | `BuiltinCall("!=")` (strict-normalised)       |
| `a && b`                    | `LogicalAnd` (short-circuit)                  |
| `a \|\| b`                  | `LogicalOr` (short-circuit)                   |
| `!a`                        | `BuiltinCall("not", [a])`                     |
| `-a` (non-literal `a`)      | `BuiltinCall("neg", [a])`                     |
| `if (c) { … } else { … }`   | `Expr::If { cond, then_branch, else_branch }` |
| `if (c) {} else if (d) {}`  | nested `Expr::If` in the else branch          |
| `while (c) { … }`           | `Stmt::While { cond, body }`                  |
| `for (let i=0; i<n; i++) {…}` | `Stmt::ForRange { var, start, stop, step, body }` |
| `for (const x of xs) { … }` | `Stmt::ForEach { var, iter, body }`           |
| `{ … }` (bare block)        | `Expr::Block`                                 |
| `function f(a, b) { … }`    | top-level `Function { name, params, body }`   |
| `return expr;` (tail only)  | `body.value = expr` (no return → `NilLit`)    |
| `(a) => a + 1`, `a => …`, `() => …` | `MakeClosure` over a synthesised `__lambda_<N>` |
| `(a) => { …; return r; }`   | same (block body, tail `return`)              |
| nested `function inner(){…}` | lifted `Function` + local `MakeClosure` binding |
| `f(1, 2)` (known function)  | `DirectCall { fn_name, args }`                |
| `g(3)` (closure value)      | `IndirectCall { target, args }`               |
| `console.log(x)`            | `BuiltinCall("print", [x])`                   |
| `[1, 2, 3]`, `[]`           | `SeqLit { items }`                            |
| `{ a: 1, "k": v }`, `{}`    | `MapLit { entries }` (id/string keys → string) |
| `xs.length`                 | `SeqLen { seq }`                              |
| `xs[i]` (non-string index)  | `SeqIndex { seq, index }`                     |
| `obj.prop`, `obj["k"]`      | `MapGet { map, key }` (string key)            |
| `xs[i] = v;`                | `Stmt::SeqSet { seq, index, value }`          |
| `obj.prop = v;`, `obj["k"] = v;` | `Stmt::MapSet { map, key, value }`       |

### Collections (M5)

- **Array / object literals** become `SeqLit` / `MapLit`. Object keys —
  identifier (`a:`) or quoted (`"k":`) — both lower to **string** map keys
  (the JS quoting distinction is purely syntactic). A parenthesised
  grouping is peeled, so `({a: 1})` lowers at statement start.
- **The bracket-vs-dot disambiguation.** The IR has both sequences and
  maps and is untyped here, so `x[i]` is structurally ambiguous. Per the
  SIR19 collections table we resolve **by access shape and key kind**:
  `.length` → `SeqLen`; every other `.prop` → `MapGet` (string key);
  `obj["k"]` (string-literal subscript) → `MapGet`; `xs[i]` (any other
  index) → `SeqIndex`. Assignment targets mirror this exactly.
- **Flat member chains.** `grid[0][1]` / `a.b.c` parse as a *single*
  `member_expression` with the accesses as a trailing token/node sequence;
  we **fold them left iteratively** (no per-segment CST recursion). A
  chained write `grid[0][1] = v` builds the receiver `grid[0]` via the read
  path, then emits the final `SeqSet`/`MapSet`.
- **Recursion safety (CWE-674).** Every element, property value, chain
  receiver, and bracket index is lowered through the depth-bounded
  `lower_expression` (`MAX_EXPR_DEPTH = 256`), so a deep `[[[[…]]]]`,
  `{a:{b:…}}`, or `p.p.p…` tower is a positioned error, not a stack
  overflow. The element/property/access iteration is non-recursive.
- **Deferred:** spread, elisions, object shorthand, computed/numeric keys,
  methods/getters/setters, `.length` assignment (a resize), and array
  methods (`.map`/`.push`/… — need runtime-library support).

### Functions, calls, and closures

- **`function` declarations** become top-level `Function`s. A two-pass
  design collects every function name (including nested ones) first, so a
  call can resolve to a `DirectCall` even for a forward reference or a
  (mutually) recursive callee.
- **Tail `return`.** A body is a `Block` whose `value` is the return.
  `return` is accepted only in tail position — the body's last statement,
  or the last statement of a branch of a tail `if` (so guard-style
  `if (base) { return b; } else { return rec; }` recursion works). An
  early `return` (one followed by more statements) is a positioned error;
  a body with no `return` yields a `NilLit` value.
- **Arrow functions and nested `function`s** lift to synthesised
  top-level `Function`s referenced by `Expr::MakeClosure`. Their free
  variables — references resolving to an enclosing function's
  local/param/capture — become `Capture`s (resolved as `Scope::Capture`
  inside the body) with matching `CaptureValue`s on the `MakeClosure`.
  Captures thread transitively through nested closures.
- **Calls** dispatch on the callee: a known module `function` →
  `DirectCall`; `console.log(x)` → `BuiltinCall("print", …)`; any other
  identifier resolving to a closure value → `IndirectCall`. A call may omit
  trailing defaulted arguments (`f(5)` against `function f(a, b = a + 1)`):
  it lowers to a **partial** `DirectCall` carrying only the present args,
  with the default filled at the call site (the validator permits this).
- **Default parameters.** A defaulted formal `name = <expr>` (both in
  `function` declarations and arrow functions) lowers to
  `Param { default: Some(<lowered expr>) }`. JS defaults are *call-time* and
  may reference *earlier* params (`function f(a, b = a + 1)`), so each
  default is lowered **inside the function frame** — a reference to an
  earlier param resolves as `Scope::Param`, matching the SIR `Param.default`
  model exactly. Declares `Feature::DefaultParams` (plus any feature the
  default expression uses). Rest `...args` and destructuring stay deferred.
- **Manifest.** Closures declare `Feature::Closures`; untyped params
  declare `Feature::DynamicTyping`; a defaulted param declares
  `Feature::DefaultParams`; a genuine call-graph cycle declares
  `Feature::MutualRecursion` (self-recursion alone does not).

### Control flow

- **`if`/`else`** lowers to an `Expr::If` (the IR's conditional is an
  expression, so a JS `if` *statement* is wrapped in an `ExprStmt`).
  Branch bodies are `Block`s — a `{ … }` block or a single unbraced
  statement. A missing `else` becomes a synthetic empty nil-valued block.
  **Else-if chains** nest naturally: the parser puts another `if` inside
  the `else`, producing a nested `Expr::If` in the outer else branch's
  tail value.
- **`while`** maps directly to `Stmt::While`.
- **C-style `for`** maps to the half-open counting `Stmt::ForRange`
  **only** when it matches the canonical shape: init `let i = <start>`,
  condition `i < <stop>` or `i <= <stop>` (the latter rewritten half-open
  to `<stop> + 1`), and update `i = i + <step>`, `i += <step>`, or `i++`.
  Anything else (a different variable across clauses, a decrement, a
  multiplicative step, a multi-variable init) is rejected with a
  positioned "non-canonical `for`" error — never silently mangled.
- **`for … of`** maps to `Stmt::ForEach` (single-identifier binding only;
  destructuring is deferred).
- **Scoping** matches the SIR validator: the loop variable is visible in
  the loop body but unresolved after the loop, and a `let` bound inside
  any control-flow body or bare block does not leak to the enclosing
  scope. Statement-block nesting is bounded by `MAX_STMT_DEPTH = 256` so
  deeply nested control flow yields an ordinary error, not a stack
  overflow.

### Variables and scope

Name resolution walks a **scope-frame stack** (one `FnScope` per
function, holding its params, captures, and locals). Top-level bindings
live in the synthetic `main` frame and resolve to `Scope::Local`; inside
a function a param resolves to `Scope::Param`, and a reference to an
enclosing frame's binding resolves to `Scope::Capture` (recording the
capture on the closure). The lowerer tracks declared names in source
order — the first sighting of `x` (`let`/`const`/`var x = …`, or a bare
`x = …` with no prior binding) emits a binding; a subsequent `x = …`
emits an `Assign` (`Feature::MutableBindings`) preserving the resolved
scope. A reference to a name that was never declared (and is not a module
function) is a positioned "unresolved name reference" error.

### Bindings: `LetStarBinding`, not `LetBinding`

`let`/`const`/`var` all lower to **`Stmt::LetStarBinding`** (sequential
`let*` semantics). The SIR validator treats a run of consecutive
`LetBinding`s as a *parallel* group whose right-hand sides cannot
reference one another, but JS declarations are sequentially scoped —
`let x = 1; const y = x + 1;` must validate — so `let*` is the correct
fit. `const`/`let`/`var` are not otherwise distinguished in v0 (the IR
models no immutability constraint), and `var` hoisting is not modelled.

### Equality is normalised to strict

JS has loose (`==`/`!=`, coercing) and strict (`===`/`!==`) equality;
the IR has only the strict-shaped `=`/`!=`. Both JS families map to the
strict IR comparison, **changing semantics** for the coercion cases
(`null == undefined` is `true` in JS but `false` here). This loss is
spec-sanctioned for v0.

### Number classification

JavaScript has a single numeric type (an IEEE-754 double). We split it
on the literal's *textual shape*: a literal with no `.` and no exponent
marker (`e`/`E`) that fits in an `i64` becomes an `IntLit`; everything
else becomes a `FloatLit`. This lets integer-heavy code round-trip
cleanly through backends that distinguish integers from floats. Hex /
octal / binary integer forms (`0x…`, `0o…`, `0b…`) and `BigInt` (`10n`)
are rejected in M1 (deferred).

### `null` vs `undefined`

Both lower to `NilLit`. The IR has a single "absence" value, so the JS
distinction is intentionally lost in v0. Note `undefined` is an
*identifier* in JS (not a keyword), so the lowerer special-cases that
exact spelling; any other bare identifier is a variable reference, which
is out of M1 scope.

## Public API

```rust
use javascript_to_semantic_ir::{compile, compile_source, JsLowerError};

// Parse + lower in one call:
let module = compile_source("42;", "demo")?;

// Or lower an already-parsed CST:
// let module = compile(&tree, "demo")?;
```

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsLowerError {
    pub message: String,
    pub line:    usize,   // 1-based; 0 when unknown
    pub column:  usize,   // 1-based; 0 when unknown
}
```

Every produced `Module`:

- wraps the top-level statements in a synthetic, exported `main`
  function whose body's tail value is the final literal expression (or
  `NilLit` for empty input);
- carries `metadata.source_language = "javascript"` and
  `metadata.sir_version = semantic_ir::CURRENT_SIR_VERSION`;
- declares **exactly** the features its literals use (`Strings` for any
  string literal, `Floats` for any float literal) so it passes
  `semantic_ir::validate` with no used-but-undeclared errors and no
  declared-but-unused warnings.

## Out of scope for M5 (deferred)

Method calls other than `console.log` and array methods (`.map`/`.push`/…,
which need runtime-library support), spread (`[...xs]` / `{...o}`), array
elisions, object shorthand / computed / numeric keys / methods / getters /
setters, `.length` *assignment* (a resize with no IR node), classes /
`this` / `new`, generators, `async`/`await`, **rest** parameters
(`...args`), destructuring, and template literals all currently return a
`JsLowerError`
describing what was rejected, with the offending node's position. So do
**early `return`** (non-tail), the remaining control-flow constructs
(`switch`, `try`/`catch`, `do … while`, labeled statements,
`break`/`continue`), and the gaps within the M2/M3 operator/assignment
families (compound assignment outside the loop-update position,
multi-binding declarations, uninitialised bindings,
bitwise/shift/exponentiation/nullish operators).
The error sites are structured so later milestones slot their handling in
at exactly the right place. See `CHANGELOG.md` for the milestone roadmap
and the full SIR19 spec at
[`code/specs/SIR19-javascript-to-semantic-ir.md`](../../../specs/SIR19-javascript-to-semantic-ir.md).

## Testing

```sh
cargo test -p javascript-to-semantic-ir
```

On Windows, prefix with the LLD linker:

```sh
CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=rust-lld cargo test -p javascript-to-semantic-ir
```

The suite includes **end-to-end `node` execution tests**
(`tests/e2e_node.rs`, gated on `node` being on `PATH`): each golden
program (factorial, fibonacci, closure-adder, mutual recursion) is lowered
to SIR, emitted back to JavaScript with the `semantic-ir-to-javascript`
backend, and run with `node` to confirm its output.
