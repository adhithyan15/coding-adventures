# SIR17 — Python → Semantic IR

## Status

Second frontend for the narrow-waist Semantic IR (after
[SIR11](SIR11-twig-to-semantic-ir.md) for Twig).  Consumes the
existing [`python-parser`](../packages/rust/python-parser/) crate and
produces a `semantic_ir::Module`.  Implemented as the Rust crate
`python-to-semantic-ir`.

Milestones implemented: **M1** (literals), **M2** (variables /
assignment / operators), **M3** (control flow), **M4** (functions,
calls, closures).  Collections / maps (M5) remain deferred.

## Pipeline

```text
Python source
   │
   ▼  python_parser::parse_python(source, "3.10")
parser::grammar_parser::GrammarASTNode (generic CST)
   │
   ▼  python_to_semantic_ir::compile_source
semantic_ir::Module                          (per SIR10 + SIR16)
```

`python-parser`'s output is a *generic* `GrammarASTNode` tree (every
node is a `rule_name: String` + `children: Vec<ASTNodeOrToken>`), not
a strongly-typed AST.  The frontend walks the tree by rule name and
extracts SIR nodes directly.  No intermediate typed-AST layer in v0 —
the per-rule extractors call into each other recursively.

## Public API

```rust
pub fn compile(
    tree:        &GrammarASTNode,
    module_name: &str,
) -> Result<semantic_ir::Module, PythonLowerError>;

pub fn compile_source(
    source:      &str,
    module_name: &str,
) -> Result<semantic_ir::Module, PythonLowerError>;  // parse + lower

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonLowerError {
    pub message: String,
    pub line:    usize,
    pub column:  usize,
}
```

The Python parser is called with version `"3.10"` by default; future
versions of this frontend may accept a version parameter.

## Subset coverage (v0 MVP)

| Python source                          | SIR lowering                                                |
|----------------------------------------|-------------------------------------------------------------|
| `42`, `-7`                             | `IntLit { value }`                                          |
| `3.14`                                 | `FloatLit { value }`                                        |
| `True`, `False`                        | `BoolLit { value }`                                         |
| `None`                                 | `NilLit`                                                    |
| `"hello"`, `'world'`                   | `StrLit { value }`                                          |
| `name` (reference)                     | `VarRef { name, scope }` with resolved scope                |
| `x = 1`                                | `LetStarBinding` (first time) or `Assign` (subsequent)      |
| `x + y`, `x - y`, `x * y`, `x / y`     | `BuiltinCall("+" / "-" / "*" / "/", [...])`                 |
| `x % y`                                | `BuiltinCall("%", [x, y])`                                  |
| `-x`                                   | `BuiltinCall("neg", [x])`                                   |
| `not x`                                | `BuiltinCall("not", [x])`                                   |
| `x == y`, `x != y`                     | `BuiltinCall("=" or "!=", [...])`                           |
| `x < y`, `x > y`, `x <= y`, `x >= y`   | `BuiltinCall("<" / ">" / "<=" / ">=", [...])`               |
| `x and y`, `x or y`                    | `LogicalAnd` / `LogicalOr`                                  |
| `if c: ... elif c2: ... else: ...`     | `If` (nested for `elif` chains)                             |
| `while c: body`                        | `While { cond: c, body }`                                   |
| `for x in range(n): body`              | `ForRange { var: x, start: 0, stop: n, step: 1, body }`     |
| `for x in range(a, b): body`           | `ForRange { var: x, start: a, stop: b, step: 1, body }`     |
| `for x in range(a, b, c): body`        | `ForRange { var: x, start: a, stop: b, step: c, body }`     |
| `for x in xs: body`                    | `ForEach { var: x, iter: xs, body }`                        |
| `def f(a, b): body`                    | `Function { name: f, params: [a, b], body }`                |
| `lambda a: expr`                       | `MakeClosure { fn_name: __lambda_N, captures, ... }`        |
| `return expr`, `return`                | `Return { value: expr | NilLit }` (see "Return" below)      |
| `f(arg1, arg2)`                        | `DirectCall` (known function) / `IndirectCall` (otherwise)  |
| `print(x)`, `len(x)`, `range(n)`       | `BuiltinCall("print" / "len" / "range", ...)`               |
| `[1, 2, 3]`                            | `SeqLit { items }`                                          |
| `xs[i]`                                | `SeqIndex { seq: xs, index: i }`                            |
| `xs[i] = v`                            | `SeqSet { seq: xs, index: i, value: v }`                    |
| `{"a": 1, "b": 2}`                     | `MapLit { entries }`                                        |
| `d[k]`                                 | `MapGet { map: d, key: k }`                                 |
| `d[k] = v`                             | `MapSet { map: d, key: k, value: v }`                       |

**Implementation note (M3 — `range` in `for` headers):** the
`for x in range(...)` rows above are realised in M3 by recognising a
literal `range(...)` call *structurally inside the `for` header* and
lowering it directly to `ForRange` (with arity 1/2/3 mapped to
`start`/`stop`/`step`, and a `range` call of wrong arity rejected).  A
non-`range` iterable lowers to `ForEach`.

**Implementation note (M4 — functions, calls, closures):** the
`def` / `lambda` / `return` / call / builtin rows above are realised in
M4.  As of M4, `range` (alongside `print` / `len`) **is** a general
expression-position `BuiltinCall` — a bare `range(n)` outside a `for`
header now lowers, in addition to the M3 `for`-header form.  `def`
lowering is **two-pass** (collect all function names first, then lower
bodies) so forward references and mutual recursion resolve to
`DirectCall`.  Nested `def`s and `lambda`s are lifted to top-level
synthesised functions with computed captures; the `Function { name: f,
params, body }` row for `def` carries `captures: []` for a top-level
`def` and a non-empty capture list for a lifted nested `def`/`lambda`.
A bare reference to a function name yields a `MakeClosure` (re-threading
the function's captures from the currently visible enclosing values).
`MutualRecursion` is declared when two top-level functions transitively
call each other (a self-recursive 1-cycle does not count).

**Implementation note (M5 — collections):** the `[...]` / `xs[i]` /
`xs[i] = v` / `{k: v}` / `d[k]` / `d[k] = v` rows above are realised in
M5, and `len(x)` is **refined**: it lowers to the dedicated `SeqLen` node
(not `BuiltinCall("len")`) so backends can emit native length access, per
the `SeqLen` node's own documentation (`len` arity must be exactly 1, and
a local/param named `len` shadows the builtin → an indirect call).  The
parser models a list display as `atom → list_expr`, a dict display as
`atom → dict_or_set_expr → dict_or_set_body → dict_body` (a set display
has *no* `dict_body` and is rejected), and a subscript as a trailing
`suffix` on a `primary` (`primary → atom suffix*`); chained suffixes
(`xs[i][j]`, `g()[0]`) fold left-to-right.

*Subscript disambiguation.*  Python overloads `[]` for list indexing and
dict lookup and the frontend has no type information, so this spec's
`xs[i] → SeqIndex` / `d[k] → MapGet` rows are realised by a purely
**syntactic** heuristic (matching the JS sibling): a **string-literal**
index lowers to `MapGet` / `MapSet` (a map key); **any other** index
lowers to `SeqIndex` / `SeqSet` (a sequence index).  This makes the
canonical idioms (`xs[0]`, `d["name"]`) correct; a dict keyed by a
variable / integer (`d[k]`, `counts[n]`) lowers as a sequence index,
which the SIR runtime's duck-typed `[]` still executes correctly (both
route through `__getitem__` / `__setitem__`).  The choice affects only the
manifest feature (`Sequences` vs `Maps`), never runtime behaviour.
Comprehensions, slicing (`xs[a:b]`), tuple / set literals, list/dict
methods, and unpacking remain deferred (positioned errors).

## Return statement

Python's `return` is a statement in the AST.  The SIR doesn't have a
`Return` node — every function body is a `Block` whose `value`
expression IS the return value.  Lowering:

- A function with a single `return expr` at the end: `body.value = expr`.
- A function ending with statements that don't return: synthesize
  `body.value = NilLit` (matches Python's implicit `None` return).
- A function with early returns: this requires lifting to a control-
  flow shape the IR can express.  For the v0 MVP, **early returns
  are not supported** — frontends reject programs with non-tail
  `return`s with a clear error pointing at the offending statement.

Future extension: add a `Return` statement and lower control flow,
or use a "linearise function via a state machine" pass.

## Scope resolution

Same model as Twig (SIR11):

- `Local` — bound by `LetBinding` / `ForRange` / `ForEach` /
  function parameters earlier in the same scope chain.
- `Param` — function parameter.
- `Capture` — captured from an enclosing function (closure body).
- `Global` — top-level value defines.
- `Builtin` — one of the predefined builtin names.

Python lacks an explicit declaration syntax for locals — `x = 1`
creates a local if `x` isn't already defined elsewhere in scope.
The frontend implements first-occurrence detection: the first
`assign(x, value)` in a function declares it, subsequent ones emit
`Assign`.  The same rule applies inside loops.

**Implementation note (M3 — block scoping):** as of M3 the frontend's
declared-name table is a **stack** (mark/rewind) mirroring the SIR
validator's `LocalEnv`, not a flat per-function set.  A loop variable
(`for i in …` / `for x in …`) and any name first bound inside a loop or
`if`-branch suite are scoped to that block only and do **not** leak past
it — exactly how the validator's `check_block` scopes them.  This keeps
the names the lowerer resolves and the names the validator accepts in
lock-step, so every lowered module round-trips through `validate`.  (A
future milestone may relax this to Python's looser "loop var survives the
loop" rule once the validator models it.)

**Implementation note (M2):** the first-occurrence *declaration* emits
a `LetStarBinding` (sequential `let*`), **not** a `LetBinding`.  The
SIR validator treats a run of consecutive `LetBinding`s as a *parallel*
group whose right-hand sides are all evaluated in the scope *before*
the group, so a later binding cannot reference an earlier one
(`x = 1` then `y = x + 1` would fail to resolve `x`).  `let*` has the
sequential semantics Python's top-to-bottom execution requires.  The
RHS is lowered before the name is added to the declared set, so a
self-referential first binding (`x = x`) correctly reports `x` as
unresolved.

`global x` and `nonlocal x` declarations are **out of scope** in v0
— the frontend rejects them.

## Closures

`lambda` and nested `def` become fresh top-level synthesised
`Function`s with computed captures, matching SIR11's approach.  Free
variables not in scope at the call site become `Capture`s.

## Top-level

Top-level Python is a sequence of statements at module scope.  The
frontend runs **all top-level statements inline as part of `main`**
(matching Python's actual execution model — there is no separate
`_init` function and no `globals` table in v0; a top-level `x = 1`
becomes a `LetStarBinding` local of `main`).  A module-level `def`
becomes a top-level `Function` entry (lifted out of `main`'s body);
`main` holds the remaining top-level statements.

```text
SIR Module {
    functions: [
        f, g, h,          // user-defined defs
        __lambda_0, ...,  // synthesised closure bodies
        main,             // contains all top-level statements
    ]
    globals: [],          // empty in v0 (top-level vars become locals of main)
}
```

## Error model

Same shape as `TwigLowerError`:

```rust
PythonLowerError {
    message: String,
    line:    usize,
    column:  usize,
}
```

Errors:

- Unresolved name reference
- Empty function body
- Early `return` (not at function tail)
- `global` / `nonlocal` declaration encountered
- Unsupported syntax (classes, exceptions, generators, comprehensions,
  decorators, multi-assign, slicing, `with`, async)
- Mismatched arity on builtins (`range` with > 3 args, `len` with > 1 arg)

## Manifest computation

The frontend declares features based on what it actually emits:

| Trigger                             | Feature added                |
|-------------------------------------|-------------------------------|
| any `FloatLit`                      | `Float` (and `OptionalTypeAnnotations` is N/A)  |
| any `Assign`                        | `MutableBindings`            |
| any `While` / `ForRange` / `ForEach`| `Loops`                       |
| any `SeqLit` / `SeqIndex` / `SeqSet`/ `SeqLen` | `Sequences`        |
| any `MapLit` / `MapGet` / `MapSet`  | `Maps`                        |
| any `LogicalAnd` / `LogicalOr`      | `ShortCircuit`               |
| any `MakeClosure` / `IndirectCall`  | `Closures`                    |
| any param with no annotation        | `DynamicTyping`              |

## Tests

`cargo test -p python-to-semantic-ir`:

- Each lowering rule has a positive unit test.
- Each error case (unsupported syntax, early return, etc.) has a
  negative test.
- Golden tests for canonical programs:
  - Factorial (function, recursion, if/else).
  - Fibonacci (function, while loop, mutation).
  - List sum (for-each over sequence).
  - Dict access.
  - Closure adder (`adder(n) → lambda x: x + n`).
- End-to-end integration through `semantic_ir::validate` (every
  lowered module passes the validator).

Coverage target ≥ 90%.

## Out of scope (deferred)

- Classes / methods / inheritance
- Exception handling
- Generators / `yield`
- `async def` / `await`
- Comprehensions
- Decorators
- Multi-target assignment / unpacking
- Slicing
- Default + keyword arguments
- String methods
- `with` / context managers
- Module-level `import`
- Sophisticated scope (PEP 3104 `nonlocal`)
- f-string formatting beyond simple `{var}` interpolation (which
  lowers to `+`-concatenation)
