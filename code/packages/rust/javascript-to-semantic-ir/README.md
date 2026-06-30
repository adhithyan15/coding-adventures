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

## Milestone status — M1 (literals)

This is the first slice of SIR19. It implements **literal lowering
only**. The supported subset:

| JavaScript source        | SIR lowering                                  |
|--------------------------|-----------------------------------------------|
| `42`, `0`, `-`-free ints | `IntLit { value }`                            |
| `3.25`, `1e3`, `1.5e-3`  | `FloatLit { value }`                          |
| `true`, `false`          | `BoolLit { value }`                           |
| `null`                   | `NilLit`                                       |
| `undefined`              | `NilLit` (distinction from `null` is lost)    |
| `"hi"`, `'hi'`           | `StrLit { value }`                            |

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

## Out of scope for M1 (deferred)

Variables, operators, control flow, functions, collections, and
template literals all currently return a `JsLowerError` describing what
was rejected, with the offending node's position. The error sites are
structured so later milestones slot their handling in at exactly the
right place. See `CHANGELOG.md` for the milestone roadmap and the full
SIR19 spec at [`code/specs/SIR19-javascript-to-semantic-ir.md`](../../../specs/SIR19-javascript-to-semantic-ir.md).

## Testing

```sh
cargo test -p javascript-to-semantic-ir
```

On Windows, prefix with the LLD linker:

```sh
CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=rust-lld cargo test -p javascript-to-semantic-ir
```
