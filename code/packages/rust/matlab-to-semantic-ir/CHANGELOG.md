# Changelog

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
- 62 tests: 54 unit tests over lowering shapes and rejected constructs, 5
  validator/capability-rejection tests (mirroring the SIR22/SIR23 core
  verification pattern), 3 end-to-end tests that actually execute lowered
  MATLAB through `semantic-ir-to-javascript` and `node` (gated on `node`
  availability).
- Marks `matlab-to-semantic-ir` done in `HML01-math-to-semantic-ir.md` §3.
