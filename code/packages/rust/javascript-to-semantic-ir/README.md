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

## Milestone status — M2 (variables, assignment, operators)

This is the second slice of SIR19. It implements literals (M1) **plus**
variable references, bindings/assignment, and unary/binary operators.
The supported subset:

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

### Variables and scope

M2 has a single flat scope: everything lives inside the synthetic
`main`, so a declared name resolves to `Scope::Local`. The lowerer
tracks declared names in source order — the first sighting of `x`
(`let`/`const`/`var x = …`, or a bare `x = …` with no prior binding)
emits a binding; a subsequent `x = …` emits an `Assign`
(`Feature::MutableBindings`). A reference to a name that was never
declared is a positioned "unresolved name reference" error.

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

## Out of scope for M2 (deferred)

Control flow, functions, collections, member access, and template
literals all currently return a `JsLowerError` describing what was
rejected, with the offending node's position — as do the gaps within
M2's own families (compound assignment, assignment to a member/index,
multi-binding declarations, uninitialised bindings, bitwise/shift/
exponentiation/nullish operators). The error sites are structured so
later milestones slot their handling in at exactly the right place. See
`CHANGELOG.md` for the milestone roadmap and the full
SIR19 spec at [`code/specs/SIR19-javascript-to-semantic-ir.md`](../../../specs/SIR19-javascript-to-semantic-ir.md).

## Testing

```sh
cargo test -p javascript-to-semantic-ir
```

On Windows, prefix with the LLD linker:

```sh
CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=rust-lld cargo test -p javascript-to-semantic-ir
```
