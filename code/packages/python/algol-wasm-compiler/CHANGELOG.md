# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Changed

- Executed ALGOL dummy statements as no-ops in `then`, `else`, `do`, and
  terminal-label statement positions.
- Added `algol60-wasm run SOURCE` for shell-level source-to-WASM execution with
  stdout forwarding, optional `_start` result reporting, and a default WASM
  instruction budget.
- Added the `algol60-wasm` command and `python -m algol_wasm_compiler` entry
  point for compiling ALGOL source files into `.wasm` modules from the shell.
- Compiled ALGOL programs now execute scalar variables through frame-backed
  WASM memory operations, including nested-block outer writes and shadowing.
- Compiled value-only integer procedures now run through generated WASM
  functions with static links, fresh recursive frames, and typed result slots.
- Compiled Phase 4 integer arrays through heap-backed descriptors with dynamic
  bounds, multidimensional row-major indexing, runtime bounds checks, and
  zero-result failure for invalid bounds or out-of-bounds accesses.
- Executed scalar by-name actuals through caller-slot storage pointers, while
  preserving IR compile-stage diagnostics for expression and array-element
  actuals that still need full eval/store thunk descriptors.
- Executed read-only integer expression actuals through tagged eval thunk
  descriptors, including repeated formal reads that observe caller-frame
  mutations between evaluations.
- Executed read-only boolean and string expression actuals through the same
  bounded word eval-thunk path, including string literals passed to formals
  that are only printed.
- Executed integer array-element by-name actuals through eval/store thunk
  helpers so repeated formal reads and assignments re-locate the current
  element, including Jensen-style index mutation between formal uses.
- Executed read-only expression thunks that read integer arrays, including
  Jensen's-device expressions and bounds failures propagated through the caller
  unwind path.
- Executed read-only expression thunks that call integer procedures, including
  nested by-name descriptor allocation and callee failure propagation through
  the caller's by-name formal read.
- Added a consolidated integer by-name acceptance test covering scalar,
  array-element, expression, nested procedure, and Jensen's-device behavior.
- Executed Phase 6 direct local labels and `goto` statements through the
  unstructured IR-to-WASM lowering path, covering forward jumps, backward
  jumps, and terminal labels.
- Executed Phase 7a local switch selections and conditional designational
  `goto` forms through the WASM path, including conditional switch entries.
- Executed Phase 7b direct nonlocal block `goto` statements through the WASM
  path with frame restoration before later block entry.
- Executed chained assignments, ALGOL-left-associative exponentiation with
  integer or real exponents, and branch-selected conditional expressions
  through the full WASM path.
- Executed nested conditional expressions in arithmetic bounds/subscripts,
  Boolean conditions, and designational `goto` targets through the full WASM
  path.
- Executed boolean `and`, `or`, and `impl` with short-circuiting RHS
  evaluation through the full WASM path, while keeping `eqv` strict.
- Executed bare no-argument typed procedure names as expression calls,
  including by-name actuals that re-evaluate through eval thunks.
- Accepted trailing and repeated semicolons in ALGOL block and compound
  statement lists.
- Returned `0` for integer `div` or `mod` by zero through the ALGOL runtime
  failure path instead of leaking a host WASM trap.
- Returned `0` for integer divide overflow, real division by zero, and
  zero-real-base negative exponentiation through the same ALGOL runtime
  failure path.
- Executed `value` whole-array parameters as callee-local descriptor and
  element copies so assignments inside the procedure do not alias the caller.
- Executed `value` label, switch, and procedure formals through the same copied
  label-id and descriptor paths as their by-name counterparts.
- Executed real-valued formal procedure calls with integer-returning procedure
  actuals by promoting the dispatched result to real before storing it.
- Executed real-returning procedures that call integer-returning procedures by
  keeping integer call results and real function returns in separate WASM
  virtual registers.
- Compiled top-level ALGOL programs without a root integer `result` scalar by
  returning `0` from the WASM `_start` wrapper while preserving the existing
  integer `result` return convention when present.
- Executed switch entries that target labels in lexical parent blocks,
  including procedure-crossing escapes to the first label in a program.
- Executed formal procedure calls whose actual procedure expects scalar
  by-name parameters, preserving lazy reads and writable caller variables.
- Executed formal procedure calls that forward whole-array arguments to actual
  procedures, preserving descriptor aliasing and existing `value` array copies.
- Executed formal procedure calls that forward label, switch, and procedure
  arguments to actual procedures.
- Executed report-style typed formal specifiers such as `integer array a;` and
  `real procedure f;` through the full WASM pipeline.
- Executed conditional switch actuals through direct calls, forwarded switch
  formals, and formal procedure dispatch, with a golden end-to-end fixture.
- Added a golden convergence fixture for real, boolean, and string by-name
  actuals through scalar storage, array-element storage, expression thunks, and
  formal-procedure forwarding.
- Accepted multiple arguments to builtin `print(...)` / `output(...)`, emitting
  each integer, boolean, real, or string argument in order through the same
  guarded output path.
- Executed forward sibling procedure calls and mutually recursive typed
  procedures by registering a block's procedure signatures before checking any
  procedure body.
- Executed switch entries that select later sibling switch declarations,
  including forward switch lists that use later typed procedure predicates.
- Executed array bounds that call later sibling typed procedures at block
  entry.
- Rejected array bounds that read arrays declared later in the same block while
  keeping earlier descriptor reads executable.
- Rejected array bounds that call procedures whose reachable bodies may access
  later same-block array descriptors before allocation, while keeping earlier
  and callee-local descriptor reads executable.
- Executed subscripted integer and real array elements as writable `for`
  statement control variables.
- Rejected formal procedure forwarding when a concrete procedure argument does
  not satisfy the nested procedure formal contract expected by the receiver.
- Executed formal procedure calls that forward another formal procedure
  parameter as a procedure argument, including read-only expression actuals and
  writable by-name actuals through the final concrete procedure.
- Executed by-name label formals through lazy label descriptors, preserving
  conditional label actual re-evaluation through direct and formal procedure
  calls while keeping `value label` formals as call-time snapshots.
- Executed by-name switch formals through lazy switch descriptors, preserving
  conditional switch actual re-evaluation through direct and formal procedure
  calls while keeping `value switch` formals as call-time snapshots.
- Added a golden designator fixture covering lazy versus value label and switch
  formals across direct, forwarded, and formal procedure calls.
- Added an executable surface audit matrix that runs representative
  grammar-backed ALGOL programs through the local WASM runtime.
- Executed recursive switch self-selection entries by routing recursive
  descriptor lookup through the switch-eval helper at runtime.
- Executed the report-style `go to` spelling through the full parser,
  type-checker, IR, and WASM pipeline.
- Executed normalized ALGOL publication symbols for relations, exponentiation,
  multiplication, real division, and boolean operators through the full WASM
  pipeline.
- Executed mixed-case standard numeric and output builtins through the full
  WASM pipeline.
- Executed standard numeric builtin functions `abs`, `sign`, and `entier`
  through the full parser, type-checker, IR, and WASM pipeline.
- Executed standard real builtin function `sqrt` through the full parser,
  type-checker, IR, and WASM pipeline with negative-domain failure returning
  `0`.
- Executed standard real builtin functions `sin`, `cos`, `arctan`, `ln`, and
  `exp` through the full parser, type-checker, IR, and WASM pipeline using the
  `compiler_math` host import ABI, with nonpositive `ln` arguments returning
  `0` through the ALGOL runtime failure path.
- Executed real exponentiation through the `compiler_math` `f64_pow` import,
  including integer-base promotion and domain failures returning `0`.
- Returned `0` for out-of-range, infinite, or NaN real values reaching
  `entier` or fixed-format real output, preventing host/WASM conversion
  exceptions from escaping end-to-end execution.
- Added a standard-real-math golden fixture covering imported real math,
  real exponentiation, and output in one end-to-end program.
- Added a convergence golden fixture that combines conditional expressions,
  exponentiation, chained assignment, by-name array summation, real arithmetic,
  and output in one end-to-end program.
- Added a front-door ALGOL source-length limit to the WASM convenience APIs so
  oversized untrusted source is rejected before parsing or semantic analysis.
- Documented and tested host-side WASM instruction budgets for compiled ALGOL
  modules so nonterminating `goto` programs can be capped by embedders.
- Fixed multi-entry switch-formal dispatch so procedure calls that receive a
  switch parameter return the selected entry label even when another label
  formal points at an earlier caller label.
- Added a full-surface golden fixture that combines `own` scalars and arrays,
  default-real arrays, nested and single-statement procedures, value/by-name
  procedure calls, label and switch formals, multiple `for` element forms,
  parenthesized conditional designational expressions, numeric labels, boolean
  operators, real arithmetic, and output in one end-to-end WASM program.
- Added golden convergence fixtures for lexical recursion, procedure-formal
  closures, nonlocal procedure `goto` unwind, dynamic multidimensional array
  bounds, mixed writable by-name scalar types, and runtime bounds-guard
  failure before output.
- Added storage-semantics convergence coverage for mixed `own` scalars and
  arrays, boolean/string `value` array copies, and value versus by-name loop
  control storage updates.
- Added edge-semantics convergence coverage for boolean/string conditional
  expressions, conditional subscripts, terminal labels, invalid switch indexes,
  array element caps, and heap exhaustion guard behavior.
- Accepted uppercase and mixed-case keywords/comments, `<>` not-equal
  relations, and double-quoted string literals through the full WASM pipeline.

## [0.1.0] - 2026-04-20

### Added

- Initial package scaffolding generated by scaffold-generator.
- Added `compile_source`, `pack_source`, and `write_wasm_file` for the
  ALGOL 60 parse/type-check/IR/WASM packaging pipeline.
