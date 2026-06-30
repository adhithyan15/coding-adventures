# Changelog

All notable changes to `javascript-to-semantic-ir` are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## 0.2.0

SIR19 milestone **M2 (variables, assignment, operators)** — builds on
M1's literal lowering.

### Added

- **Variable references.**  A bare identifier resolves to
  `Expr::VarRef { name, scope }`.  M2 has a single flat scope (everything
  lives in the synthetic `main`), so a declared name resolves to
  `Scope::Local`.  `undefined` continues to collapse to `NilLit`; any
  other *undeclared* identifier is a positioned
  `JsLowerError { message: "unresolved name reference `…`" }`.
- **Bindings.**  `let x = e;`, `const x = e;`, and `var x = e;` all lower
  to a binding statement.  We emit **`Stmt::LetStarBinding`** (sequential
  `let*`), not `Stmt::LetBinding`: the validator treats a run of
  consecutive `LetBinding`s as a *parallel* group whose RHSs cannot see
  one another, but JS `let`/`const` are sequentially scoped, so a
  perfectly ordinary `let x = 1; const y = x + 1;` must validate.  `let*`
  matches JS exactly.  `const`/`let`/`var` are not distinguished in v0
  (the IR models no immutability constraint); `var` hoisting is not
  modelled (binding emitted at its source position).
- **Re-assignment.**  `x = e;` to an already-declared name lowers to
  `Stmt::Assign { scope: Local }` (declares `Feature::MutableBindings`).
  A first bare `x = e;` with no prior declarator still binds (JS
  implicitly creates the binding), matching the resolution model.
- **Binary operators** (flat left-associative chains fold left into
  nested `BuiltinCall`s):
  - arithmetic `+ - * / %` → `BuiltinCall("+"/"-"/"*"/"/"/"%")`;
  - comparison `< > <= >=` → `BuiltinCall("<"/">"/"<="/">=")`.
- **Equality normalisation.**  Both `==` *and* `===` → `BuiltinCall("=")`,
  and both `!=` *and* `!==` → `BuiltinCall("!=")`.  This is a **deliberate
  semantic change**: the IR has only the strict-shaped comparison, so the
  loose-equality coercion cases (e.g. `null == undefined`, `true` in JS)
  become `false`.  Spec-sanctioned for v0 (SIR19 "Equality
  normalisation").
- **Logical operators.**  `&&` → `Expr::LogicalAnd`, `||` →
  `Expr::LogicalOr` — short-circuit nodes (declares
  `Feature::ShortCircuit`), **not** builtins (a builtin would eagerly
  evaluate both operands).
- **Unary operators.**  `!x` → `BuiltinCall("not", [x])`; `-x` →
  `BuiltinCall("neg", [x])`.  `-<numeric literal>` is **constant-folded**
  into a negative literal (`-5` → `IntLit(-5)`, `-3.25` →
  `FloatLit(-3.25)`), keeping the spec's `-7 → IntLit` row exact.
- **Bounded operand recursion.**  The precedence-spine peel stays
  iterative; only genuine operand descent recurses, capped at
  `MAX_EXPR_DEPTH = 256` so an adversarial deeply-nested expression
  yields an ordinary positioned error rather than a stack-overflow abort.
- 17 new unit tests (34 total): variable resolution incl. let-then-
  reassign and first-bare-assignment, unresolved-name error, every
  arithmetic/comparison operator, equality strict-normalisation,
  left-associativity and precedence, `&&`/`||` short-circuit nodes (and a
  guard that they are *not* builtins), unary `!`/`-`/`-literal` fold, a
  cross-referencing-bindings validation test (the parallel-`let` trap),
  and mixed-program validate round-trips.

### Changed

- Minor version bump `0.1.0 → 0.2.0` (additive, backward-compatible).
- The two M1 error-path tests that asserted operators and bare
  identifiers were *rejected* now assert they lower correctly.

### Deferred

Still **out of scope after M2** and returning a clear positioned
`JsLowerError`, tracked against the SIR19 spec:

- **M3 — control flow:** `if`/`else`, `while`, `for` (`ForRange`),
  `for-of` (`ForEach`).
- **M4 — functions & closures:** `function` declarations, arrow
  functions (`MakeClosure`), `return`, calls (`DirectCall` /
  `IndirectCall`), `console.log` → `BuiltinCall("print", …)`.
- **M5 — collections:** array literals (`SeqLit`), indexing
  (`SeqIndex`), `.length` (`SeqLen`), object literals (`MapLit`),
  member/`[]` access (`MapGet`).
- **Operator/assignment gaps within M2's families:** compound assignment
  (`+=`, `-=`, …), assignment to a member/index target (`obj.x = …`,
  `xs[i] = …`), multi-binding declarations (`let a = 1, b = 2;`),
  uninitialised bindings (`let x;`), bitwise / shift / exponentiation /
  nullish-coalescing operators, and other prefix unaries (`+`, `~`,
  `typeof`, `void`, `delete`).
- **Template literals** (backtick strings) — distinct token/rule; will
  desugar to concatenation in a later milestone.
- **Non-decimal numeric forms:** hex (`0x…`), octal (`0o…`), binary
  (`0b…`), and `BigInt` (`10n`) literals remain rejected.
- Everything in the SIR19 spec "Out of scope (deferred)" section:
  classes, exceptions, generators, `async`/`await`, destructuring,
  spread/rest, default parameters, ES modules, `eval`, regex.

## 0.1.0

First release — SIR19 milestone **M1 (crate skeleton + literal
lowering)**.

### Added

- Crate skeleton: `Cargo.toml` (path deps on `semantic-ir`,
  `coding-adventures-javascript-parser`, `parser`, `lexer`), `BUILD` /
  `BUILD_windows`, `README.md`, and this changelog.
- Public API:
  - `compile(tree: &GrammarASTNode, module_name: &str) -> Result<Module, JsLowerError>`
    — lower an already-parsed JavaScript CST.
  - `compile_source(source: &str, module_name: &str) -> Result<Module, JsLowerError>`
    — parse (with the `es2020` grammar) then lower, surfacing parse
    failures as `JsLowerError`s with a best-effort `line:column`.
  - `JsLowerError { message, line, column }`
    (`Debug`/`Clone`/`PartialEq`/`Eq`, plus `Display` + `std::error::Error`).
- Literal lowering from the **generic** `GrammarASTNode`:
  - integer-shaped number literal → `IntLit`;
  - decimal / exponent number literal → `FloatLit`;
  - `true` / `false` → `BoolLit`;
  - `null` **and** `undefined` → `NilLit` (the JS distinction is
    intentionally lost in v0);
  - string literal (single- or double-quoted) → `StrLit`.
- Synthesises an exported `main` function whose body's tail value is the
  final top-level literal (or `NilLit` for empty source).
- Stamps `metadata.source_language = "javascript"` and
  `metadata.sir_version = CURRENT_SIR_VERSION`, and emits a feature
  manifest declaring exactly the observed features (`Strings`,
  `Floats`), so every produced module passes `semantic_ir::validate`.
- 17 unit tests: one per literal kind, `compile_source` structural
  checks, a validate round-trip, and error paths (operator expression,
  bare identifier, parse failure, position extractor).

### Deferred

The following are explicitly **out of scope for M1** and currently
return a clear `JsLowerError`. They are scheduled for later milestones,
tracked against the SIR19 spec
(`code/specs/SIR19-javascript-to-semantic-ir.md`):

- **M2 — variables & operators:** variable references (`VarRef`),
  `let`/`const`/`var` (`LetBinding`), re-assignment (`Assign`),
  arithmetic / comparison / logical operators, unary `!`/`-`, loose vs.
  strict equality normalisation.
- **M3 — control flow:** `if`/`else`, `while`, `for` (`ForRange`),
  `for-of` (`ForEach`).
- **M4 — functions & closures:** `function` declarations, arrow
  functions (`MakeClosure`), `return`, calls (`DirectCall` /
  `IndirectCall`), `console.log` → `BuiltinCall("print", …)`.
- **M5 — collections:** array literals (`SeqLit`), indexing
  (`SeqIndex`), `.length` (`SeqLen`), object literals (`MapLit`),
  member/`[]` access (`MapGet`).
- **Template literals** (backtick strings, e.g. `` `a ${x} b` ``):
  deferred — these are a distinct token/rule from plain strings and will
  desugar to `+`-concatenation in a later milestone.
- **Non-decimal numeric forms:** hex (`0x…`), octal (`0o…`), binary
  (`0b…`), and `BigInt` (`10n`) literals are rejected in M1.
- Everything in the SIR19 spec "Out of scope (deferred)" section:
  classes, exceptions, generators, `async`/`await`, destructuring,
  spread/rest, default parameters, ES modules, `eval`, regex.
