# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Changed

- Lowered ALGOL scalar locals through planned activation-frame slots instead
  of source-variable virtual registers.
- Added frame-memory metadata, frame header setup/teardown, static-link
  traversal, and `LOAD_WORD`/`STORE_WORD` scalar accesses for the WASM path.
- Bounded phase-2 frame memory to one WASM page so large or crafted semantic
  frame plans fail before downstream WASM data allocation.
- Lowered value-only integer procedure calls to generated IR functions with
  explicit static-link/value-argument registers, procedure frame allocation,
  procedure-name result slots, and bounded recursive frame usage.
- Reserved runtime state bytes in the phase-3 frame-memory cap so programs
  that cannot fit their root frame are rejected before WASM lowering.
- Lowered Phase 4 integer arrays to heap-backed descriptors with dynamic
  bound evaluation, row-major strides, checked element loads/stores, bounded
  aggregate element counts, and block-lifetime heap restoration.
- Lowered scalar by-name actuals as storage pointers so by-name formal reads
  and writes execute through the caller's scalar slot, while expression and
  array-element actuals continue to report explicit future-thunk diagnostics.
- Lowered read-only integer expression actuals to tagged eval thunk descriptors
  with bounded call-scoped heap allocation and generated eval dispatch that
  re-evaluates against the caller frame on every formal read.
- Covered read-only boolean and string expression actuals through the same word
  eval-thunk path used by integer expressions.
- Lowered integer array-element by-name actuals to tagged descriptors with
  generated eval/store helpers that re-compute the element address on every
  formal read or assignment.
- Allowed read-only expression eval thunks to read integer arrays, including
  helper-side bounds failure propagation back into the caller unwind path.
- Allowed read-only expression eval thunks to call integer procedures, with a
  runtime helper-depth marker for propagating nested procedure failures back to
  the caller's by-name formal read.
- Documented the completed integer Phase 5 by-name subset and its remaining
  full-ALGOL exclusions.
- Lowered Phase 6 direct local `goto` statements to generated ALGOL IR labels
  and `JUMP` instructions, while preserving diagnostics for nonlocal and
  designational forms that need Phase 7.
- Lowered Phase 7a local conditional designational expressions and switch
  selections into local IR jumps, including one-based switch dispatch and
  runtime failure for out-of-range switch indexes.
- Lowered Phase 7b direct nonlocal block `goto` statements by unwinding exited
  block frames before jumping to the outer ALGOL label.
- Lowered chained assignments, branch-selected conditional expressions, and
  ALGOL-left-associative exponentiation for numeric bases with integer or real
  exponents.
- Lowered boolean `and`, `or`, and `impl` through short-circuiting control
  flow while keeping `eqv` strict.
- Lowered bare no-argument typed procedure names as expression calls, including
  use inside read-only by-name eval thunks.
- Lowered integer `div` and `mod` zero-divisor checks through the existing
  runtime-failure guard so WASM execution returns `0` instead of trapping.
- Lowered integer divide-overflow checks, real division zero-divisor checks,
  and zero-real-base negative exponent checks through the same runtime-failure
  guard path.
- Lowered `value` whole-array parameters by allocating a callee-local copy of
  the array descriptor, bounds metadata, and element storage at procedure entry.
- Stored label, switch, and procedure formals as copied ids or descriptor
  pointers at procedure entry even when they are declared in `value` mode.
- Allowed real-valued formal procedure dispatchers to accept integer-returning
  procedure actuals and promote their result through the existing real coercion
  path.
- Emitted real-returning procedures, eval thunks, and procedure-parameter
  dispatchers through the WASM backend's dedicated f64 result register so they
  can also call integer-returning procedures safely.
- Allowed top-level ALGOL programs without a root integer `result` scalar to
  compile by returning `0` from the WASM `_start` wrapper.
- Lowered switch declaration entries that target labels in lexical parent
  blocks through the same frame-unwind and pending-goto paths as direct gotos.
- Tagged pending procedure-crossing label ids so label id `0` no longer
  collides with the no-pending-goto sentinel.
- Lowered formal procedure dispatch arguments as lazy storage pointers or
  thunk descriptors, allowing actual procedures with scalar by-name parameters
  to read or assign through the original argument.
- Lowered whole-array arguments through formal procedure dispatchers by passing
  descriptor pointers to actual procedures that declare matching array formals.
- Lowered subscripted integer and real array elements as writable `for`
  statement control variables.
- Lowered label, switch, and procedure arguments through formal procedure
  dispatchers by forwarding label ids and descriptor pointers.
- Lowered report-style typed formal specifiers such as `integer array a;` and
  `real procedure f;` through the existing array/procedure formal paths.
- Lowered conditional switch designator actuals by selecting and forwarding the
  chosen switch descriptor through concrete and formal procedure calls.
- Preserved concrete procedure ids in formal procedure call-shape metadata so
  nested procedure-parameter contracts are checked before IR lowering.
- Lowered formal procedure parameters passed as procedure arguments to another
  formal procedure call, preserving the forwarded procedure descriptor instead
  of treating the bare formal name as a scalar expression.
- Lowered by-name label formals as lazy label descriptors, so conditional
  label actuals are re-evaluated when the formal is used by `goto`, while
  `value label` formals retain call-time snapshot behavior.
- Lowered recursive switch self-selection through runtime switch-eval descriptor
  dispatch rather than compile-time descriptor expansion.
- Lowered `go to` statements through the same direct and designational goto
  paths as the compact `goto` spelling.
- Lowered normalized ALGOL publication symbols for relations, exponentiation,
  and boolean operators through the existing ASCII/keyword operator paths.
- Lowered standard numeric builtin functions `abs`, `sign`, and `entier` using
  existing integer/f64 IR operations.
- Lowered standard real builtin function `sqrt` to integer-to-real promotion,
  a negative-domain runtime failure guard, and the `F64_SQRT` IR opcode.
- Lowered standard real builtin functions `sin`, `cos`, `arctan`, `ln`, and
  `exp` to integer-to-real promotion and the corresponding imported f64 math
  IR opcodes, with nonpositive `ln` guarded by the runtime-failure path.
- Lowered real exponentiation to integer-to-real promotion and the imported
  `F64_POW` IR opcode, with NaN results routed through the runtime-failure
  path.
- Guarded real-to-integer truncation before emitting `I32_TRUNC_FROM_F64`, so
  oversized, infinite, or NaN real values return through ALGOL's runtime
  failure path instead of leaking WASM traps or host exceptions.
- Added configurable generated-state limits for eval thunks, conditional label
  sets, loop label sets, switch dispatch states, and output helper label sets,
  with targeted `CompileError` diagnostics when lowering would exceed them.
- Copied every switch-entry value into a stable result register in the
  switch-formal evaluation helper, so multi-entry switch parameters now return
  the label selected by the runtime index instead of depending on the last
  compiled entry register.
- Lowered `<>` as the same not-equal comparison as `!=` for numeric, boolean,
  and string operands.

## [0.1.0] - 2026-04-20

### Added

- Initial package scaffolding generated by scaffold-generator.
- Added `compile_algol` lowering from checked ALGOL 60 ASTs to `compiler_ir`
  with structured `if` and `for` label shapes for the WASM backend.
