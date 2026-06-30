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

## Milestone status — M5 (collections)

Top-level statements run inline in a synthesised `main` function whose
block value is the program's final top-level expression (or `NilLit`
for an empty program / a trailing assignment).  A `def` at any level is
lifted to a **top-level `Function`** (the module gains a real function
table); `lambda`s and nested `def`s are lifted to synthesised functions
with computed captures.  M5 adds **collections** — list & dict literals,
indexing (`x[i]`), `len`, and subscript assignment — on top of M1–M4
(`def`, tail `return`, `lambda`, calls, closures).

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

### Functions, calls & closures (M4)

| Python source                          | SIR lowering                                          | Feature declared       |
|----------------------------------------|-------------------------------------------------------|------------------------|
| `def f(a, b): …`                       | top-level `Function { name f, params [a, b], body }`  | `DynamicTyping`†       |
| `return expr` (tail)                   | function body `value = expr`                          | —                      |
| `return` / no return (tail)            | function body `value = NilLit` (implicit `None`)      | —                      |
| `return expr` (non-tail / early)       | **error** — "early return not supported in v0"        | —                      |
| `lambda a: expr`                       | `MakeClosure { fn_name __lambda_N, captures }`        | `Closures`             |
| nested `def` (returned / referenced)   | lifted `Function` + `MakeClosure` with captures       | `Closures`             |
| `f(args)` — `f` a known function       | `DirectCall { fn_name f, args }`                      | —                      |
| `f(args)` — `f` a closure value        | `IndirectCall { target VarRef, args }`                | `Closures`             |
| `print(x)`, `len(x)`, `range(n)`       | `BuiltinCall("print" / "len" / "range", …)`           | —                      |
| `is_even`/`is_odd` cross-calling       | (call-graph cycle of length ≥ 2)                      | `MutualRecursion`      |

† a `def`/`lambda` with parameters declares `DynamicTyping` (the subset
has no parameter annotations).

**Two-pass design.**  A first pass collects **every** function name
(top-level and nested) into a flat table, so a call to a function defined
*later* in the file — and **mutual recursion** — resolve to `DirectCall`.
The second pass lowers each body.

**`return` is tail-only.**  A SIR function body is a `Block` whose `value`
IS the return value, so a tail `return expr` sets that value (and a bare
tail `return` / falling off the end yields `NilLit`).  A tail `if` whose
branches each `return` (`if c: return a else: return b`) lowers with each
branch in function-tail position, becoming an `Expr::If`.  A **non-tail**
`return` (followed by more statements, or nested in a loop / non-tail
branch) is rejected with a positioned error — the IR has no `Return`
node, so early returns are deferred.

**Closures & capture.**  A `lambda` / nested `def` is lifted to a fresh
top-level function; its **free variables** (names the body reads that are
not its own params / locals and that resolve to an *enclosing* local /
param / capture) become `Capture`s, threaded through `MakeClosure` as
`CaptureValue`s and resolved inside the body as `Scope::Capture`.  Names
that resolve to a global / top-level function / builtin need **no**
capture.  A bare reference to a function name yields a `MakeClosure`
(re-threading its captures), so closures can be returned or passed.
Capture order is deterministic (alphabetical).  Closure bodies reuse the
`MAX_BLOCK_DEPTH` / `MAX_EXPR_DEPTH` guards, so recursion stays bounded.

### Collections (M5)

| Python source              | SIR lowering                                  | Feature declared |
|----------------------------|-----------------------------------------------|------------------|
| `[a, b, c]`, `[]`          | `SeqLit { items }`                            | `Sequences`      |
| `xs[i]` (non-string index) | `SeqIndex { seq, index }`                     | `Sequences`      |
| `xs[i] = v`                | `SeqSet { seq, index, value }`                | `Sequences`      |
| `len(xs)`                  | `SeqLen { seq }`†                             | `Sequences`      |
| `{"a": 1, "b": 2}`, `{}`   | `MapLit { entries }`                          | `Maps`           |
| `d["k"]` (string index)    | `MapGet { map, key }`                         | `Maps`           |
| `d["k"] = v`               | `MapSet { map, key, value }`                  | `Maps`           |

† `len` lowers to the dedicated `SeqLen` node (preferred over
`BuiltinCall("len")` so backends can emit native length access); arity
must be exactly 1, and a local/param named `len` shadows the builtin.

**Subscript disambiguation.**  Python overloads `[]` for both list
indexing and dict lookup, and the frontend has no types.  The rule is
purely syntactic: a **string-literal** index is a *map key*
(`MapGet` / `MapSet`); any other index is a *sequence index*
(`SeqIndex` / `SeqSet`).  So `xs[0]` and `d["name"]` lower as expected,
while `d[k]` / `counts[n]` (a variable / integer key) lower as a sequence
index.  The choice only affects the manifest feature (`Sequences` vs
`Maps`) — the SIR runtime's duck-typed `[]` executes both identically.

**Suffix folding.**  A `primary` is `atom suffix*`; trailing call /
subscript suffixes fold **left-to-right**, so `xs[i][j]`, `g()[0]`, and
mixed chains lower correctly.  Every element / index / key is lowered
depth-bounded (`MAX_EXPR_DEPTH`), so a deep `[[[…]]]` / `{a:{b:…}}` /
`xs[xs[xs[…]]]` tower returns a clean positioned error rather than
overflowing the stack.

The manifest declares **exactly** the features observed.  Module
metadata records `source_language = "python"` and
`sir_version = semantic_ir::CURRENT_SIR_VERSION`.  Every lowered module
passes `semantic_ir::validate`.

### Deferred (later milestones)

Everything past M5 returns a clear positioned `PythonLowerError`:

- list / dict **comprehensions**, **slicing** (`xs[a:b]`), **tuple** /
  **set** literals, and list/dict **methods** (`.append` / `.keys` /
  `.get` — these need the SIR runtime-library)
- default / keyword arguments, `*args` / `**kwargs`, multi-level capture
  chaining (capturing a variable two scopes up)
- tuple / multi-target `for` (`for k, v in …`), multi-target / chained
  assignment, attribute targets, bitwise operators, the power operator
  (`**`)
- and the full SIR17 "out of scope" list (classes, exceptions,
  generators, decorators, `with` / `try`, `async`, imports,
  `global` / `nonlocal`, f-strings).

## Testing & coverage

```sh
cargo test -p python-to-semantic-ir
```

The suite (82 tests) covers M1–M3 (literals; operators with
left-associativity / precedence; variable resolution and first-occurrence
assignment; short-circuit nodes; `if` / `elif` / `else`, `while`,
`for`-range / `for`-each with block scoping) plus the M4 surface:

- **functions** — `def` lifting with params (`Scope::Param`), tail-return
  vs no-return→`NilLit`, body statements + tail return;
- **early-return rejection** — a `return` followed by more statements, or
  nested in an `if` branch, errors with a position;
- **calls** — `DirectCall` (incl. forward references resolved by the
  first pass), `BuiltinCall` (`print` / `len` / `range`), and
  `IndirectCall` through a closure value;
- **closures** — `lambda` → `MakeClosure` + synthesised function; capture
  of an enclosing local; non-capture of globals / functions; nested-`def`
  capture of an enclosing param with a returned closure;
- **mutual recursion** detected (and self-recursion correctly *not*
  flagged);
- a `validate` round-trip across the M4 programs, and
- **executed end-to-end** round-trips — factorial, fibonacci, a closure
  adder, a capturing lambda, and mutual recursion are lowered to SIR,
  re-emitted to Python via `semantic-ir-to-python`, and run with the
  system `python` (gated on availability), asserting on stdout.

```sh
cargo test -p python-to-semantic-ir
```

This exercises ≥ 90% of the M4 surface.
