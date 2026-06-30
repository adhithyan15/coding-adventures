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

## Milestone status — M3 (control flow)

The whole program is wrapped in a synthesised `main` function whose
block value is the program's final top-level expression (or `NilLit`
for an empty program, or when the last statement is an assignment);
earlier top-level statements become `ExprStmt`s (bare expressions) or
binding / `Assign` / loop statements.  M3 adds `if` / `elif` / `else`,
`while`, and `for` (`range` and iterables).

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

### Control flow (M3)

| Python source                          | SIR lowering                                          | Feature declared |
|----------------------------------------|-------------------------------------------------------|------------------|
| `if c: …`                              | `If { cond, then, else: nil-block }`                  | —                |
| `if c: … else: …`                      | `If { cond, then, else }`                             | —                |
| `if c: … elif d: … else: …`            | `If { c, …, else: If { d, …, else } }` (nested)       | —                |
| `while c: body`                        | `While { cond, body }`                                | `Loops`          |
| `for i in range(n): body`              | `ForRange { var, start 0, stop n, step 1, body }`     | `Loops`          |
| `for i in range(a, b): body`           | `ForRange { var, start a, stop b, step 1, body }`     | `Loops`          |
| `for i in range(a, b, c): body`        | `ForRange { var, start a, stop b, step c, body }`     | `Loops`          |
| `for x in xs: body`                    | `ForEach { var, iter xs, body }`                      | `Loops`          |

`if` is modelled as a SIR **expression** (`Expr::If`), so a *trailing*
`if` becomes its enclosing block's value, while an `if` in non-tail
position becomes a `Stmt::ExprStmt` wrapping the `If`.  An `elif` chain
folds right-to-left into nested `If`s (each `elif` is the `else_branch`
of the clause before it); a missing `else` synthesises an empty block
whose value is `NilLit`.  `if` declares **no** feature — it is a SIR v0
construct.

Each loop / branch **suite** lowers to a `Block` (statements then a
value).  A `for` header's `range(...)` is recognised structurally and
its arity mapped to `(start, stop, step)`; `range` with zero or more
than three arguments is rejected ("range with wrong arity").  Any other
iterable becomes a `ForEach`.

**Scoping.**  The loop variable (`i` / `x`) is a `Scope::Local` bound
**inside the body only**, and a name first bound inside a loop or
`if`-branch body does **not** leak past the block — the lowerer's
declared-name table is a stack mirroring the validator's `LocalEnv`, so
lowering and validation agree.  Nested control flow is bounded by
`MAX_BLOCK_DEPTH` (companion to `MAX_EXPR_DEPTH`), turning pathological
nesting into a clean error rather than a native stack overflow.

The manifest declares **exactly** the features observed.  Module
metadata records `source_language = "python"` and
`sir_version = semantic_ir::CURRENT_SIR_VERSION`.  Every lowered module
passes `semantic_ir::validate`.

### Deferred (later milestones)

Everything past M3 returns a clear `PythonLowerError`
(`"unsupported: <rule> (deferred …)"`) so later milestones slot in
where the error is raised today:

- functions (`def`), lambdas, calls (`f(...)`, `print` / `len`
  builtins; `range` is recognised only in `for` headers, not as a
  general call yet) — M4
- sequences, maps, indexing, comprehensions — M5
- tuple / multi-target `for` (`for k, v in …`), multi-target / chained
  assignment, attribute / subscript targets, bitwise operators, the
  power operator (`**`)
- and the full SIR17 "out of scope" list (classes, exceptions,
  generators, decorators, `with` / `try`, `async`, imports,
  `global` / `nonlocal`, f-strings).

## Testing & coverage

```sh
cargo test -p python-to-semantic-ir
```

The suite (57 tests) covers: one positive test per literal kind; the
M2 operators (each arithmetic, comparison, unary, and logical form),
left-associativity and precedence; variable resolution,
let-then-reference, and let-vs-reassign first-occurrence; the
short-circuit-node shape; the M3 control flow — `if` / `elif` / `else`
nesting (incl. no-else and elif-without-else nil branches, trailing vs
statement-position `if`), `while` (with body re-assignment), `for`-range
at all three arities plus variable bounds and the zero-/four-arg arity
errors, `for`-each, loop-variable and branch-local scope non-leakage,
and nested control flow; top-level structure (empty program, `ExprStmt`
+ value split, metadata, minimal manifest); a `validate` round-trip
across every literal, M2, and M3 construct; and error paths (unresolved
name, self-reference, `global`, `def` / `with` deferral, parse error,
error position).  This exercises ≥ 90% of the M3 surface.
