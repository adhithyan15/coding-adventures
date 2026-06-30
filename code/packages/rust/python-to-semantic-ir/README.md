# python-to-semantic-ir

Python CST → narrow-waist Semantic IR (**SIR17**).  The second
frontend for the SIR10 narrow-waist IR, after
[`twig-to-semantic-ir`](../twig-to-semantic-ir/).

This crate consumes the *generic* `GrammarASTNode` concrete syntax
tree produced by the
[`coding-adventures-python-parser`](../python-parser/) crate and emits
a [`semantic_ir::Module`](../semantic-ir/) that any SIR v0 backend can
consume.

## Pipeline

```text
Python source
   │
   ▼  coding_adventures_python_parser::parse_python(src, "3.10")
parser::grammar_parser::GrammarASTNode      (generic CST)
   │
   ▼  python_to_semantic_ir::compile
semantic_ir::Module                         (per SIR10 + SIR16)
```

The Python parser's output is a generic CST — every node is a
`rule_name: String` plus `children: Vec<ASTNodeOrToken>`, and every
precedence level of the expression grammar is its own rule.  The
frontend walks the tree by rule name; there is no intermediate
typed-AST layer.

## Public API

```rust
pub fn compile(
    tree:        &GrammarASTNode,
    module_name: &str,
) -> Result<semantic_ir::Module, PythonLowerError>;

pub fn compile_source(
    source:      &str,        // parsed at Python version "3.10"
    module_name: &str,
) -> Result<semantic_ir::Module, PythonLowerError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonLowerError {
    pub message: String,
    pub line:    usize,
    pub column:  usize,
}
```

`compile_source` parses then lowers; both parse and lower failures are
surfaced as `PythonLowerError`.

## Milestone status — M1 (literals)

M1 is the crate skeleton plus **literal lowering**.  The whole program
is wrapped in a synthesised `main` function whose block value is the
program's final top-level expression (or `NilLit` for an empty
program); earlier top-level expressions become `ExprStmt`s.

| Python source        | SIR lowering            | Feature declared |
|----------------------|-------------------------|------------------|
| `42`, `-7`           | `IntLit { value }`      | —                |
| `3.25`, `-2.5`       | `FloatLit { value }`    | `Floats`         |
| `True`, `False`      | `BoolLit { value }`     | —                |
| `None`               | `NilLit`                | —                |
| `"hi"`, `'world'`    | `StrLit { value }`      | `Strings`        |

`-7` / `-2.5` are constant-folded: a `factor( "-", numeric-literal )`
becomes a negative literal.  Unary minus on a *non*-literal is
deferred.

The manifest declares **exactly** the features observed (`Floats` only
when a float literal appears, `Strings` only when a string appears), so
a minimal int-only program declares no extra features.  Module metadata
records `source_language = "python"` and
`sir_version = semantic_ir::CURRENT_SIR_VERSION`.  Every lowered module
passes `semantic_ir::validate`.

### Deferred (later milestones)

Everything past literals returns a clear `PythonLowerError`
(`"unsupported in M1: <rule>"`) so later milestones slot in where the
error is raised today:

- variable references (`x`) and assignment (`x = 1`) — M2
- arithmetic / comparison / boolean operators — M3
- control flow (`if` / `while` / `for`) — M3
- functions (`def`), lambdas, calls — M4
- sequences, maps, indexing — M5
- and the full SIR17 "out of scope" list (classes, exceptions,
  generators, comprehensions, decorators, `with`, `async`, imports).

## Testing & coverage

```sh
cargo test -p python-to-semantic-ir
```

The suite has one positive test per literal kind (int, negative int,
float, negative float, `True`, `False`, `None`, double- and
single-quoted strings), top-level structure tests (empty program,
multi-statement `ExprStmt` + value split, metadata, minimal manifest),
a `validate` round-trip across every literal shape, and error-path
tests (assignment, bare name, operator, parse error, error position).
This exercises ≥ 90% of the M1 surface.
