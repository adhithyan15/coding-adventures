# Changelog

All notable changes to `python-to-semantic-ir` are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to semantic versioning.

## 0.4.0 — 2026-06-30

Milestone **M4**: functions, calls, and closures — `def`, tail-position
`return`, `lambda`, function calls, and free-variable-captured closures
(SIR17 Python → Semantic IR frontend, B4).

### Added

- **`def f(params): suite` → a top-level `Function`.**  Lowering is now
  **two-pass**: a first pass collects **every** function name (top-level
  and nested `def`s) into a flat table, so calls — including *forward
  references* and *mutual recursion* — resolve to `DirectCall` regardless
  of textual order; the second pass lowers each body.  Parameters are in
  scope as `Scope::Param`.  A `def` with parameters declares
  `Feature::DynamicTyping` (the subset has no parameter annotations).
- **`return` (tail position only).**  A function body is a `Block` whose
  `value` IS the return value, so a **tail** `return expr` sets
  `body.value = expr`; a bare tail `return` or falling off the end yields
  `body.value = NilLit` (Python's implicit `None`).  A tail `if` whose
  branches each end in a `return` (the canonical
  `if c: return a else: return b` shape) is lowered with each branch in
  *function-tail* position, so those returns become the branch values of
  an `Expr::If`.  A **non-tail** (early) `return` — one nested in control
  flow or followed by more statements — is **rejected** with a positioned
  `PythonLowerError("early return not supported in v0 …")` pointing at the
  offending `return`, per the SIR17 spec (the IR has no `Return` node).
- **`lambda params: expr` → a synthesised top-level `Function` +
  `Expr::MakeClosure`.**  Each lambda is gensym'd `__lambda_<N>`; the use
  site emits a `MakeClosure` referencing it.
- **Nested `def` → lifted to a top-level synthesised function** with the
  same closure treatment as a lambda.  A **bare reference** to a nested
  function's name yields a `MakeClosure` (re-threading its captures from
  the currently visible enclosing values), so the closure can be
  `return`ed / passed.
- **Free-variable capture.**  For a lambda / nested `def`, the body's free
  names (bare references minus the body's own params and locally assigned
  names) that resolve to an **enclosing local / param / capture** become
  `Capture`s — threaded through `MakeClosure` as `CaptureValue`s and
  resolved inside the synthesised function as `Scope::Capture`.  Names that
  resolve to a global / top-level function / builtin need **no** capture
  (they are reachable directly).  Captures are emitted in deterministic
  (alphabetical) order for reproducible output.
- **Calls.**  `f(args)` lowers to `Expr::DirectCall` when `f` is a known
  function name (and not shadowed by a same-named local value),
  `Expr::BuiltinCall` for the builtins `print` / `len` / `range` (now a
  general expression-position builtin, not only the `for`-header form),
  and `Expr::IndirectCall` through a `VarRef` when `f` is a local / param /
  captured **closure handle**.  Argument expressions are lowered eagerly.
- **Manifest** now declares `Feature::Closures` whenever the module emits
  a retained `MakeClosure`, an `IndirectCall`, or a function with
  captures; and `Feature::MutualRecursion` when two top-level functions
  transitively call each other (a 1-cycle — plain self-recursion — does
  **not** count).  Bounded closure-body recursion reuses the
  `MAX_BLOCK_DEPTH` / `MAX_EXPR_DEPTH` guards.
- 25 new unit tests (82 total), including **executed end-to-end**
  round-trips (Python → SIR → Python via the `semantic-ir-to-python`
  backend, run with the system `python`, gated on availability): factorial
  (recursion + tail if/else), fibonacci (while + mutation), a closure
  adder, a capturing lambda, and mutual recursion.  Structural unit tests
  cover `def`/params, tail-return vs no-return→nil, early-return rejection,
  `DirectCall` vs `BuiltinCall` vs `IndirectCall`, forward-reference
  resolution, lambda + capture, nested-def + capture, mutual-recursion
  detection, default-parameter rejection, and a validator round-trip set.

### Fixed

- **Depth-bounded the three M4 pre-lowering CST walks** so the public
  `compile` cannot overflow the native (uncatchable) stack on a
  pathologically deep input.  These walks run *before* the depth-guarded
  lowering, so they previously bypassed the `MAX_BLOCK_DEPTH` /
  `MAX_EXPR_DEPTH` guards:
  - `collect_function_names` (pass-1 def-name collection) now threads a
    block-nesting `depth` capped at `MAX_BLOCK_DEPTH`;
  - `collect_free_names` (free-variable scan) and `walk_for_targets` /
    `descendant_assign_targets` / `collect_suite_bound_names` (bound-name
    scan) now thread an expression-nesting `depth` capped at
    `MAX_EXPR_DEPTH`.
  Each returns a clean positioned `PythonLowerError("… nesting too deep …")`
  past the cap, mirroring the lowering guards.  Two regression tests
  (84 total) build a 400-deep `def` tower and a 400-deep expression inside
  a function body — run on an enlarged stack so the (separately unguarded)
  parser survives to reach the lowerer — and assert a clean "too deep"
  error rather than a crash.

### Changed

- `compile` / `compile_source` now lower `def` / `lambda` / `return` /
  calls.  The module's function table holds user `def`s, synthesised
  closure bodies (`__lambda_<N>`, lifted nested defs), and `main` (the
  top-level statements).  The per-function name-resolution state is now a
  `FunctionCtx` (params / captures / a local stack) rather than a single
  shared declared-name stack, so each function resolves names in its own
  scope.
- Added a **dev-dependency** on `semantic-ir-to-python` for the executed
  end-to-end tests.

### Deferred

Still out of scope after M4; each returns a clear positioned
`PythonLowerError`:

- **M5+** — sequences, maps, indexing and indexed assignment,
  comprehensions.
- Default / keyword arguments, `*args` / `**kwargs`, decorators, classes,
  exceptions, generators, slicing, string methods, imports, `async`,
  `global` / `nonlocal`, tuple / multi-target assignment, multi-level
  capture chaining (capturing a variable two scopes up).

## 0.3.0 — 2026-06-30

Milestone **M3**: control flow — `if` / `elif` / `else`, `while`, and
`for` (`range` and iterables) (SIR17 Python → Semantic IR frontend, B3).

### Added

- **`if` / `elif` / `else`.**  The parser flattens the whole construct
  into a single `if_stmt` whose children are an ordered token+node stream
  (`if` cond `:` suite, then zero or more `elif` cond `:` suite, then an
  optional `else` `:` suite).  We collect the `(cond, suite)` clauses plus
  the optional `else` suite, then fold **right-to-left** so each `elif`
  becomes the `else_branch` of the clause before it:
  `if c1: B1 elif c2: B2 else: B3` ⇒
  `If { c1, B1, else: If { c2, B2, else: B3 } }`.  An absent `else` becomes
  an empty `else_branch` block whose value is `NilLit` (SIR requires both
  branches; the false path yields nil, matching Python).  Each suite lowers
  to a `Block`.  Because Python's `if` is a statement but SIR models it as
  an `Expr::If`, a **trailing** `if` becomes `main`'s (or an enclosing
  suite's) block *value*; an `if` in non-tail position becomes a
  `Stmt::ExprStmt` wrapping the `If`.  `if` adds **no** manifest feature
  (it is a SIR v0 construct).
- **`while c: body`** → `Stmt::While { cond, body }`, declaring
  `Feature::Loops`.
- **`for x in range(...): body`** → `Stmt::ForRange { var, start, stop,
  step, body }`, recognising the literal `range(...)` call and mapping its
  arity: `range(n)` → start `0`, stop `n`, step `1`; `range(a, b)` →
  start `a`, stop `b`, step `1`; `range(a, b, c)` → all three.  A `range`
  call with zero or more than three arguments is rejected with a positioned
  "range with wrong arity" error.  Bounds may be arbitrary expressions
  (literals or variable references), not just literals.
- **`for x in <iterable>: body`** → `Stmt::ForEach { var, iter, body }`
  for any non-`range` iterable.  The iterable is lowered in the
  *enclosing* scope (before the loop variable is bound), matching the
  validator.
- **Loop-variable + block scoping.**  The lowerer's declared-name table
  is now a **stack** (was a flat `HashSet`) with `mark()`/`rewind()`,
  mirroring the SIR validator's `LocalEnv`.  A loop variable (`i` / `x`)
  is bound as a `Scope::Local` **inside the body only**; a name first
  bound inside a loop or `if`-branch body does **not** leak past the
  block.  This keeps the names the lowerer resolves and the names the
  validator accepts in lock-step, so every lowered module still
  round-trips through `semantic_ir::validate`.
- **Bounded block recursion.**  A new `MAX_BLOCK_DEPTH` guard (companion
  to `MAX_EXPR_DEPTH`) caps statement-block nesting, so a pathological
  tower of nested loops / `if`s fails with a clean positioned
  `PythonLowerError` instead of a native (uncatchable) stack overflow.
- **Manifest** now declares `Feature::Loops` whenever the module emits a
  `While` / `ForRange` / `ForEach`, keeping the declared manifest exactly
  matched to what the module emits.
- 22 new unit tests (57 total): if / elif / else nesting (including
  no-else and elif-without-else nil branches, trailing-value vs
  statement-position `if`), `while` (with body re-assignment), `for`-range
  at all three arities plus variable bounds and the zero-/four-arg arity
  errors, `for`-each (including iterable-resolved-before-loop-var),
  loop-variable and branch-local scope non-leakage, nested control flow
  (`if` in `while`, `for` in `for`), still-deferred `def` / `with`, and an
  extended validator round-trip set over control-flow programs.

### Changed

- `compile` / `compile_source` now accept the M3 control-flow constructs;
  a `statement` may now wrap a `compound_stmt` (`if_stmt` / `while_stmt` /
  `for_stmt`) in addition to a `simple_stmt`.  Suites are lowered into
  `Block`s via a shared `lower_suite` helper.

### Deferred

Still out of scope after M3; each returns a clear
`PythonLowerError("unsupported: <rule> (deferred …)")` at the exact site a
later milestone will handle it:

- **M4+** — `def` functions, `lambda` / closures, calls (`f(...)`,
  `print` / `len` builtins; `range` is recognised only in `for` headers,
  not as a general call yet).
- **M5+** — sequences, maps, indexing and indexed assignment,
  comprehensions.
- Tuple / multi-target `for` (`for k, v in …`), `with` / `try`, classes,
  exceptions, generators, decorators, slicing, default/keyword args,
  string methods, imports, `async`, `global` / `nonlocal`, f-strings.

## 0.2.0 — 2026-06-30

Milestone **M2**: variable references, assignment, and unary/binary
operators (SIR17 Python → Semantic IR frontend, B2).

### Added

- **Variable references.**  A bare `Name` atom (`x`) lowers to a
  `VarRef { name, scope }`.  Scope resolution follows the SIR17 model:
  a name bound earlier in the current (module / `main`) scope resolves
  to `Scope::Local`; an unbound name raises a positioned
  `PythonLowerError("unresolved name `x`")` (no builtins are wired up
  until calls arrive in a later milestone).
- **Assignment with first-occurrence detection.**  `x = expr` tracks
  declared names per scope: the **first** assignment to a name declares
  it (emits `Stmt::LetStarBinding`), and a **subsequent** assignment to
  an already-declared name re-binds it (emits `Stmt::Assign`, declaring
  `Feature::MutableBindings`).  `LetStarBinding` (sequential `let*`) is
  used rather than `LetBinding` so a later RHS can see an earlier
  binding — the SIR validator treats consecutive `LetBinding`s as a
  *parallel* group whose RHS cannot see one another, which would break
  Python's top-to-bottom execution (`x = 1` then `y = x + 1`).  The RHS
  is lowered before the name is declared, so `x = x` correctly reports
  `x` as unresolved.
- **Operators**, recognised by turning each precedence rule into a
  small operator matcher (still bounded by the M1 `MAX_EXPR_DEPTH`
  depth-tracked peel — every recursive descent increments `depth`, so
  pathologically deep input yields a clean error, never a stack
  overflow):
  - arithmetic `+ - * / %` (rules `arith` / `term`) →
    `BuiltinCall("+"/"-"/"*"/"/"/"%", [lhs, rhs])`, left-associative;
  - comparison `== != < > <= >=` (rule `comparison` / `comp_op`) →
    `BuiltinCall(op, [lhs, rhs])`, mapping `==`→`"="` and `!=`→`"!="`
    per SIR17, the rest keeping their literal spelling;
  - unary `not x` (rule `not_expr`) → `BuiltinCall("not", [x])`;
  - unary `-x` (rule `factor`) → `BuiltinCall("neg", [x])`, with
    `-<numeric literal>` still constant-folded to a negative literal
    (carried from M1); unary `+x` is the identity (operand returned
    unchanged);
  - `x and y` / `x or y` (rules `and_expr` / `or_expr`) →
    `LogicalAnd` / `LogicalOr` short-circuit nodes (left-nested for
    chains), declaring `Feature::ShortCircuit`.
- **Manifest** now also declares `Feature::ShortCircuit` (any `and`/`or`)
  and `Feature::MutableBindings` (any re-assignment) in addition to M1's
  `Floats` / `Strings`, keeping the declared manifest exactly matched to
  what the module emits.  Every lowered module still round-trips through
  `semantic_ir::validate`.
- 16 new unit tests (35 total): operator lowering (each arithmetic /
  comparison / unary / logical form), left-associativity and
  precedence, variable resolution, let-then-reference, let-vs-reassign
  first-occurrence, unresolved-name and self-reference errors,
  short-circuit-node shape, and an extended validator round-trip set.

### Changed

- `compile` / `compile_source` now accept the M2 constructs above; the
  M1 "unsupported in M1" errors for assignment, variable references, and
  operators are replaced by real lowering.  Remaining unsupported forms
  return `PythonLowerError("unsupported: <rule> (deferred …)")`.

### Deferred

Still out of scope after M2; each returns a clear
`PythonLowerError("unsupported: <rule> (deferred …)")` at the exact
site a later milestone will handle it:

- **M3+** — control flow (`if` / `elif` / `else`, `while`, `for` /
  `range`), `def` functions, `lambda` / closures, calls
  (`f(...)`, `print` / `len` / `range` builtins).
- **M4+** — sequences, maps, indexing and indexed assignment.
- Multi-target / tuple / chained assignment (`a, b = …`, `a = b = …`),
  attribute / subscript assignment targets, the bitwise operators
  (`& | ^ << >> ~`), and the power operator (`**`).
- Full SIR17 "out of scope" set: classes, exceptions, generators,
  comprehensions, decorators, slicing, default/keyword args, string
  methods, `with`, imports, `async`, `global` / `nonlocal`, f-strings.

## 0.1.0 — 2026-06-30

Milestone **M1**: crate skeleton + literal lowering (SIR17 Python →
Semantic IR frontend, B1).

### Added

- Public API per the SIR17 spec:
  - `compile(tree: &GrammarASTNode, module_name: &str) -> Result<Module, PythonLowerError>`
  - `compile_source(source: &str, module_name: &str) -> Result<Module, PythonLowerError>`
    (parses at Python `"3.10"`, then lowers).
  - `PythonLowerError { message, line, column }` (`Debug`, `Clone`,
    `PartialEq`, `Eq`), with `Display`/`Error` impls.
- Literal lowering, peeling the parser's deep precedence-rule onion
  down to the `atom` token:
  - integer literals → `IntLit` (incl. constant-folded `-7`)
  - float literals → `FloatLit` (declares `Feature::Floats`); incl.
    constant-folded `-2.5`
  - `True` / `False` → `BoolLit`
  - `None` → `NilLit`
  - string literals (single- and double-quoted) → `StrLit` (declares
    `Feature::Strings`); the parser pre-resolves escapes.
- Synthesised `main` function: the final top-level expression becomes
  the block value (or `NilLit` when the program is empty); earlier
  top-level expressions become `ExprStmt`s.
- Manifest declares **exactly** the observed features; module metadata
  records `source_language = "python"` and
  `sir_version = CURRENT_SIR_VERSION`.  Every lowered module passes
  `semantic_ir::validate`.
- 19 unit tests (one per literal kind, top-level structure, validator
  round-trip, and error paths) covering ≥ 90% of the M1 surface.
- Package scaffolding: `Cargo.toml` (path deps on `semantic-ir`,
  `coding-adventures-python-parser`, `parser`, `lexer`), `BUILD` /
  `BUILD_windows`, `README.md`, this changelog.  Added the crate to the
  `code/packages/rust` workspace members list.

### Deferred

Out of scope for M1; each returns a clear
`PythonLowerError("unsupported in M1: <rule>")` so later milestones
slot in at the same site:

- **M2** — variable references (`x`) and assignment (`x = 1`,
  `assign_suffix`), first-occurrence `LetBinding` vs `Assign`.
- **M3** — arithmetic / comparison / boolean operators, control flow
  (`if` / `while` / `for`), unary minus on non-literals.
- **M4** — `def` functions, `lambda`/closures, calls.
- **M5** — sequences, maps, indexing.
- Full SIR17 "out of scope" set: classes, exceptions, generators,
  comprehensions, decorators, multi-target assignment, slicing,
  default/keyword args, string methods, `with`, imports, `async`,
  `global`/`nonlocal`, f-strings.
