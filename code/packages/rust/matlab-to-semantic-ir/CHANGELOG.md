# Changelog

## [Unreleased]

### Added

- **`tests/oracle.rs`: the first oracle/golden test in the whole HML01
  track (spec §7)** — for a small corpus of MATLAB programs, runs the SAME
  computation through (a) `matlab-runtime` (this frontend's own sibling
  interpreter, ground truth) and (b) `matlab_to_semantic_ir::compile_source`
  → `semantic_ir::Module` → `semantic_ir_to_javascript::compile` → a real
  `node` process, and asserts the two agree. Every prior `e2e_node.rs`-style
  test anywhere in this track (this crate's own, plus
  `wolfram-to-semantic-ir`/`macsyma-to-semantic-ir`'s) only proved the
  compiled JS *runs without crashing* — none diffed it against the
  language's own native runtime, which is the actual definition of "oracle
  testing" per HML01 §7. Marks MATLAB's oracle test done in HML01 §5's
  Stream A rollout summary (Octave/APL/J's remain open follow-ons).
  - Corpus (7 cases, all passing, `node` genuinely invoked — not skipped):
    literal arithmetic operator precedence, a bare comparison, `if`/`else`,
    an `elseif` chain, a `for`-loop accumulator, matrix multiplication, and
    elementwise scalar broadcast (the last two are real SIR22 array/matrix
    cases, not just scalars — the actual point of Stream A).
  - **Two confirmed bugs in `matlab-runtime` itself** (out of scope to fix
    in this test-only PR), found because building the harness required
    reading its output byte-for-byte: (1) its `disp` builtin
    (`src/builtins.rs`) is a no-op that discards its argument and returns an
    invisible empty array — `eval("disp(7)\n")` returns `"ans =\n\n\n\n"`,
    never `"7"`; neither that crate's own tests nor `matlab-repl`'s ever
    exercise `disp`, relying instead on MATLAB's other, working
    implicit-display echo convention, which is what this oracle harness
    uses for ground truth instead. (2) `eval.rs`'s statement dispatch has no
    `func_def` arm at all — a program containing a `function ... end`
    definition cannot be run by `matlab-runtime` even though this crate's
    own `e2e_node.rs` already compiles and runs one. (3) indexed assignment
    (`A(2) = 9;`) is rejected by `eval_expr_or_assign` ("assignment target
    must be a variable") even though it is ordinary MATLAB and this crate's
    own `e2e_node.rs` already round-trips it. (2) and (3) mean the
    corresponding compiled-path constructs simply have no ground truth to
    diff against yet, not that they are broken.
  - **Three confirmed bugs/gaps surfaced in this crate and
    `semantic-ir-to-javascript`** by cross-checking against real MATLAB
    semantics (also out of scope to fix here — test infrastructure only;
    see `tests/oracle.rs`'s module doc for full root-cause writeups):
    (1) integer-literal division floors instead of true-dividing (`7 / 2`
    compiles to `3`, not MATLAB's `3.5`) — `number_literal_expr` lowers a
    decimal-point-free literal to `Expr::IntLit`, and the JS backend's
    shared `divide()` helper (built for Ruby's `Integer#/`, which really
    does floor) floors whenever both operands are integer-valued, with no
    per-source-language override, and MATLAB has no integer type at all.
    (2) unary minus on a power expression gives `NaN` instead of the
    correct value (`-2 ^ 2` should be `-4`) — `^`/`.^` unconditionally
    lower to the SIR22 array-domain `ElementwiseOp::Pow` (no literal-only
    scalar fast path, unlike `+`/`-`/`*`), so even two literal operands
    produce an NDArray-shaped object, and `neg`'s codegen applies a bare
    native `-(...)` to it, which coerces to `NaN`. (3) `try_logical`
    (`src/lower.rs`) never calls `self.observed.add(Feature::ShortCircuit)`
    for `&&`/`||`/`&`/`|`, so any MATLAB program using them fails
    `semantic_ir::validate()` outright.
  - **One severe, previously-unnoticed correctness bug**, given its own
    dedicated, always-informative test
    (`known_bug_while_loop_accumulator_terminates_after_one_iteration`): a
    `while` loop whose condition variable is also updated via non-literal
    (variable-involving) arithmetic runs its body exactly **once** instead
    of to convergence — a silent wrong *computation*, not merely a wrong
    *display* like the bugs above. Root cause: the accumulator becomes an
    NDArray-shaped object after its first `ElementwiseOp` update (same
    root cause as the power/`neg` bug), and the loop's own condition then
    compiles to a native `<`/`>` comparison against that object, which is
    unconditionally `false` — so the loop silently stops after one
    iteration with no error, no validator issue, and a plausible-looking
    wrong answer. `for`-loop accumulators are not immune to the underlying
    wrapping, they are just structurally shielded from it (their own
    termination test is index-driven, never accumulator-driven).

### Changed

- **`semantic-ir-to-javascript` now accepts and correctly compiles the
  SIR22 array/matrix modules this frontend produces** — no code change in
  this crate; that backend gained real codegen for `NDArrays`/`MatrixOps`/
  `ArrayColumnMajor` (previously deferred/rejected). Updated
  `tests/test_validator.rs`'s three tests from "the backend rejects this"
  to "the backend accepts this," and added four real `node`-execution
  tests to `tests/e2e_node.rs` proving actual MATLAB source using matrix
  multiplication, elementwise scalar broadcast (`A .* 2`), indexed
  assignment, and range+transpose all compile and run correctly — not
  just that the module passes validation.

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

- **Correctness: a `while` loop whose condition variable was also a
  non-literal arithmetic accumulator ran its body exactly once, not to
  convergence.** Found and diagnosed by `tests/oracle.rs`'s
  `known_bug_while_loop_accumulator_terminates_after_one_iteration` (the
  oracle test added just above, same PR). This was a silent wrong
  *computation* — no error, no validator issue, no crash — and the most
  severe finding of that oracle-harness effort. Root cause traced end to
  end: `expr_is_known_scalar` (this crate's own scalar/array
  disambiguation heuristic) only treats a *literal*-derived expression as
  provably scalar, so `n = n + 1` (where `n` is a variable) always lowers
  to the SIR22 `Expr::ElementwiseOp` path regardless of what `n` actually
  holds; `semantic-ir-to-javascript`'s `ElementwiseOp` codegen always
  returns an NDArray-shaped `{ shape, data }` object even for a
  logically-scalar result; and that backend's shared `numOf` helper (used
  by every comparison and by `neg`/`minus`/`mod`) only unwrapped a tagged
  `SirFloat` box, not a scalar (`shape.length === 0`) NDArray — so `n < 10`
  compiled to `__Sir.lt(n, 10)`, which coerced the wrapped `n` through
  `ToPrimitive` to `NaN`, and `NaN < 10` is silently `false`. **Fixed in
  `semantic-ir-to-javascript` (this crate's own `src/` needed no change):
  `numOf` now also unwraps a scalar NDArray** — see that crate's own
  0.40.0 CHANGELOG entry for the full write-up, including the
  `numof_unwraps_scalar_ndarray_for_comparison_and_negation` regression
  test and confirmation that the same fix also resolves the
  unary-minus-on-power bug (`-2 ^ 2` giving `NaN`) documented below, for
  free. `tests/oracle.rs`'s `known_bug_while_loop_accumulator_
  terminates_after_one_iteration` is renamed to
  `while_loop_accumulator_converges_correctly` and now asserts the correct
  converged value (`10`), with its doc comment rewritten to describe the
  fix instead of the bug; the corresponding bullet in that file's module
  doc comment is updated to say FIXED.

- **Correctness: unary minus on a power expression gave `NaN`, not the
  correct value** (`-2 ^ 2` should be `-4`; documented alongside the
  while-loop bug above, same root cause, same fix — see that entry and
  `semantic-ir-to-javascript`'s 0.40.0 CHANGELOG entry). Confirmed fixed
  end to end by re-running `compile_source("disp(-2 ^ 2)\n")` →
  `semantic_ir_to_javascript::compile` → `node`: prints `-4`.

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
