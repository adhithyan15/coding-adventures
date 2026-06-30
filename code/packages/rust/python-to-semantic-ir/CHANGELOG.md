# Changelog

All notable changes to `python-to-semantic-ir` are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to semantic versioning.

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
