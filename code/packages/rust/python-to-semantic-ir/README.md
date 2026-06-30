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

## Milestone status — M2 (variables, assignment, operators)

The whole program is wrapped in a synthesised `main` function whose
block value is the program's final top-level expression (or `NilLit`
for an empty program, or when the last statement is an assignment);
earlier top-level statements become `ExprStmt`s (bare expressions) or
binding / `Assign` statements.

### Literals (M1, still supported)

| Python source        | SIR lowering            | Feature declared |
|----------------------|-------------------------|------------------|
| `42`, `-7`           | `IntLit { value }`      | —                |
| `3.25`, `-2.5`       | `FloatLit { value }`    | `Floats`         |
| `True`, `False`      | `BoolLit { value }`     | —                |
| `None`               | `NilLit`                | —                |
| `"hi"`, `'world'`    | `StrLit { value }`      | `Strings`        |

`-7` / `-2.5` are constant-folded: a `factor( "-", numeric-literal )`
becomes a negative literal.

### Variables & assignment (M2)

| Python source                | SIR lowering                              | Feature declared   |
|------------------------------|-------------------------------------------|--------------------|
| `x` (bound earlier)          | `VarRef { name, scope: Local }`           | —                  |
| `x` (not bound)              | error: `unresolved name `x``              | —                  |
| `x = 1` (first occurrence)   | `LetStarBinding { name, value }`          | —                  |
| `x = 2` (already declared)   | `Assign { name, scope: Local, value }`    | `MutableBindings`  |

Assignment uses **first-occurrence detection** (per scope): the first
`x = …` declares the name, later ones re-bind it.  The frontend emits
`LetStarBinding` (sequential `let*`) for the declaration so a later
RHS can see an earlier binding — `x = 1` then `y = x + 1` resolves `x`
correctly.  The RHS is lowered before the name is declared, so `x = x`
reports `x` as unresolved (matching Python's `NameError`).

### Operators (M2)

| Python source                | SIR lowering                              | Feature declared   |
|------------------------------|-------------------------------------------|--------------------|
| `a + b`, `a - b`             | `BuiltinCall("+" / "-", [a, b])`          | —                  |
| `a * b`, `a / b`, `a % b`    | `BuiltinCall("*" / "/" / "%", [a, b])`    | —                  |
| `a == b`, `a != b`           | `BuiltinCall("=" / "!=", [a, b])`         | —                  |
| `a < b`, `>`, `<=`, `>=`     | `BuiltinCall("<" / ">" / "<=" / ">=", …)` | —                  |
| `not x`                      | `BuiltinCall("not", [x])`                 | —                  |
| `-x` (non-literal)           | `BuiltinCall("neg", [x])`                 | —                  |
| `+x`                         | identity (operand returned unchanged)     | —                  |
| `a and b`                    | `LogicalAnd { lhs, rhs }`                 | `ShortCircuit`     |
| `a or b`                     | `LogicalOr { lhs, rhs }`                  | `ShortCircuit`     |

Binary operators are left-associative and respect Python precedence
(`a + b * c` → `a + (b * c)`).  `and` / `or` use the dedicated
short-circuit nodes rather than `BuiltinCall`, so the right operand is
not eagerly evaluated.  Operator lowering reuses M1's depth-tracked
peel (`MAX_EXPR_DEPTH`): every recursive descent increments the depth
counter, so pathologically deep input yields a clean `PythonLowerError`
instead of a native stack overflow.

The manifest declares **exactly** the features observed.  Module
metadata records `source_language = "python"` and
`sir_version = semantic_ir::CURRENT_SIR_VERSION`.  Every lowered module
passes `semantic_ir::validate`.

### Deferred (later milestones)

Everything past M2 returns a clear `PythonLowerError`
(`"unsupported: <rule> (deferred …)"`) so later milestones slot in
where the error is raised today:

- control flow (`if` / `while` / `for`) — M3
- functions (`def`), lambdas, calls (`f(...)`, `print` / `len` /
  `range`) — M3/M4
- sequences, maps, indexing — M4/M5
- multi-target / chained assignment, attribute / subscript targets,
  bitwise operators, the power operator (`**`)
- and the full SIR17 "out of scope" list (classes, exceptions,
  generators, comprehensions, decorators, `with`, `async`, imports,
  `global` / `nonlocal`, f-strings).

## Testing & coverage

```sh
cargo test -p python-to-semantic-ir
```

The suite (35 tests) covers: one positive test per literal kind; the
M2 operators (each arithmetic, comparison, unary, and logical form),
left-associativity and precedence; variable resolution,
let-then-reference, and let-vs-reassign first-occurrence; the
short-circuit-node shape; top-level structure (empty program,
`ExprStmt` + value split, metadata, minimal manifest); a `validate`
round-trip across every literal and M2 construct; and error paths
(unresolved name, self-reference, `global`, parse error, error
position).  This exercises ≥ 90% of the M2 surface.
