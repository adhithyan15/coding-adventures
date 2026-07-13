# Changelog

## [Unreleased]

### Fixed

- **Correctness: `Feature::Floats` was never observed for a `FloatLit`.**
  `number_literal_expr` was a free function with no access to the
  lowerer's feature-tracking state, so a module containing a float
  literal never declared `Feature::Floats` in its manifest even though
  `semantic-ir/src/validator.rs`'s `check_expr` requires it for every
  `Expr::FloatLit` node — any MATLAB (and, transitively, Octave) program
  with a float literal failed `semantic_ir::validate()`. Found while
  implementing `macsyma-to-semantic-ir` (which cross-checked its own
  `Feature::Floats` handling against every sibling frontend). Fixed by
  converting `number_literal_expr` into an instance method that calls
  `self.observed.add(Feature::Floats)` on every `FloatLit`-constructing
  branch; added a regression test asserting a float-literal program both
  validates and is accepted by the JS backend.

## [0.1.0] - 2026-07-11

### Added

- Initial `matlab-to-semantic-ir` frontend crate (HML01 §3), the first to
  target SIR22 (array/matrix domain): `compile`/`compile_source` lowering
  `coding-adventures-matlab-parser`'s `GrammarASTNode` CST into a
  `semantic_ir::Module`.
- Supported: literals (int/float/string), assignment (`LetStarBinding` on
  first occurrence, `Assign` on re-assignment), arithmetic
  (`+ - * / \ ^` and their dotted elementwise forms), comparisons, logical
  `&& || & &`, unary `+ - ~`, ranges (`a:b`, `a:step:b`), transpose (`'`
  and `.'`), matrix literals (`ArrayLit`), indexing (read → `IndexGet`,
  write → `Stmt::IndexSet`) with 1-based → 0-based translation at lowering
  time, `if`/`elseif`/`else`, `while`, `for i = a:b`, single/zero-output
  function definitions and calls, and `disp` (mapped onto the shared SIR
  `print` builtin).
- A conservative, purely syntactic scalar/array disambiguation heuristic
  for MATLAB's shape-polymorphic operators (`expr_is_known_scalar`):
  literal-derived operands take a plain `BuiltinCall`, everything else
  takes the SIR22 `ElementwiseOp`/`MatMul` path.
- Explicit, disclosed scope limits (each rejected with a clear
  `MatlabLowerError`, never silently mis-lowered): stepped/matrix-valued
  `for` loops, `end`-relative indexing, matrix division (`/`/`\` between
  non-scalars — `array-runtime` has no linear-solve kernel), matrix power,
  multi-output functions, nested function definitions,
  `break`/`continue`/`return` (semantic-ir has no early-exit control-flow
  node at all yet), `switch`/`try`/`global`/`persistent`, cell arrays,
  anonymous functions, auto-vivification on indexed assignment, and
  chained assignment.
- 66 tests: 57 unit tests over lowering shapes, rejected constructs, and
  the DoS-guard regressions below, 5 validator/capability-rejection tests
  (mirroring the SIR22/SIR23 core verification pattern), 3 end-to-end
  tests that actually execute lowered MATLAB through
  `semantic-ir-to-javascript` and `node` (gated on `node` availability).
- Marks `matlab-to-semantic-ir` done in `HML01-math-to-semantic-ir.md` §3.

### Fixed (security review, before first push)

- **DoS: unbounded native-stack recursion on a flat arithmetic chain.**
  MATLAB's grammar collapses a flat run of `+`/`-`/`*`/... (no parens) into
  one CST node with many children, so a long unparenthesized chain never
  trips the ordinary grammar-nesting depth guard. Two compounding bugs,
  both confirmed to crash (SIGABRT) on a 60,000-term chain during review:
  (1) `build_additive`/`build_multiplicative` re-derived each operand's
  scalar-ness by calling `expr_is_known_scalar` on the *entire
  already-accumulated* left tree at every fold step — O(chain length)
  stack on the final step alone — fixed by tracking scalar-ness
  incrementally (O(1) per step) instead; (2) even with (1) fixed, folding
  N operands left-associatively still builds an N-deep binary `Expr` tree,
  and that depth is what every later recursive pass over it (the
  validator, any backend, even `Drop`) pays for regardless of how cheaply
  it was built — fixed by capping the operand *count* itself
  (`check_chain_length`, applied to `additive`/`multiplicative`/
  `comparison`/`logical_or`/`logical_and`) at `MAX_EXPR_DEPTH`, rejecting a
  pathological chain before building anything. `expr_is_known_scalar` also
  gained its own depth cap as defense in depth.
- **DoS (masked, not yet independently exploitable): index/call arguments
  reset the expression-depth counter instead of threading it.**
  `lower_index_args`/`lower_call_args`/`lower_one_index_arg` called the
  depth-*resetting* `lower_expr` instead of continuing the caller's depth,
  so a chain of nested indexing/calls (`A(A(A(...))))`) never accumulated
  against `MAX_EXPR_DEPTH` — each level silently restarted its own budget.
  Currently masked by `coding-adventures-matlab-parser`'s own independent
  nesting limit (which rejects sufficiently deep source first), but that
  is a different crate's protection, not this one's — fixed by threading
  `depth + 1` through all three functions via `lower_expr_d`, mirroring
  `python-to-semantic-ir`'s `lower_expr_in` pattern.
- **Test-only, LOW severity: predictable temp-file path in
  `tests/e2e_node.rs`.** The end-to-end harness wrote to a predictable
  path under the shared system temp directory via `std::fs::write`, which
  follows an existing symlink; switched to
  `OpenOptions::new().write(true).create_new(true)`, which fails instead
  of following one.
