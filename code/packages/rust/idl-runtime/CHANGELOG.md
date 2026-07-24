# Changelog

## [0.1.0] - 2026-07-24

### Added

- Initial release — **MA-12d** of the IDL frontend (spec
  [`MA12`](../../../specs/MA12-idl-language.md)): a tree-walking evaluator
  over `array-runtime`, making the IDL lexer/parser (`idl-lexer`/
  `idl-parser`, MA-12b/MA-12c) executable.
- `Interpreter` (persistent session) + `feed`/`eval` entry points.
  Auto-print (Implied Print) semantics confirmed directly against NV5
  Geospatial's own documentation: assignment is silent, a bare top-level
  expression statement auto-prints, and Implied Print does **not** fire
  inside a `PRO`/`FUNCTION` body.
- **`IdlValue::{Num(Array), Str(String)}`** (MA12 §2) — IDL's own small
  value enum, deliberately not reusing another language's (`MatValue`,
  `ScilabValue`), the same "own enum, same pattern" precedent MA10
  established for Scilab.
- **`IdlCallable`**, built on Q's `QFn::Lambda` scope-frame precedent
  (MA11 §2) per MA12 §3's own explicit instruction, layering on:
  - **Two separate namespaces** (`procs`/`funcs`) — real IDL allows the
    same name to be both a `PRO` and a `FUNCTION` simultaneously;
    `idl-parser`'s CST already routes each call site
    (`procedure_call_stmt` vs. an expression's `call_suffix`) to its own
    dispatch table.
  - **Keyword-argument binding**: positional call-site args bind by
    position to a callable's positional parameters; `KEYWORD=value`/
    `/KEYWORD` call-site args bind by name against `KEYWORD=local_var_name`
    header declarations (the local variable name may differ from the
    call-site keyword spelling, MA12 §4's own literal example). An omitted
    parameter (positional OR keyword) is left genuinely unbound, not
    defaulted — the load-bearing property `N_ELEMENTS(kw) EQ 0` relies on.
  - **No automatic outer/global visibility inside a call** — unlike Q's
    lambda (which falls back to the global frame), MA12 §4 defers
    `COMMON` blocks and states a routine reads/writes only its own
    parameters/keywords/locals; `lookup`/`assign` only ever touch the
    environment stack's **top** frame, never searching down to an outer
    caller's frame the way `q-runtime`'s does.
  - `N_ELEMENTS`'s special-cased "is this bare name currently bound at
    all" check (MA12 §3), evaluated *before* the ordinary
    evaluate-then-error argument path, so an omitted optional keyword
    answers `0` instead of raising an "undefined variable" error.
- **Control flow**: `IF...THEN...ELSE` (single-statement and
  `BEGIN...ENDIF/ENDELSE/END` block forms), `FOR v=init,limit[,step] DO`,
  `WHILE expr DO`, `REPEAT...UNTIL`, `BREAK`, `CONTINUE`, `RETURN`
  (validated against the enclosing routine's own kind — a value-carrying
  `RETURN` inside a `PRO`, or a bare `RETURN` inside a `FUNCTION`, is a
  clean error) — modeled with a small `Flow` signal
  (`Normal`/`Break`/`Continue`/`Return`) threaded up through nested
  statement execution, the ordinary shape a tree-walking imperative
  interpreter uses (no Q precedent, since Q is expression-only).
- **Assignment**, including subscripted targets, and the **full subscript
  surface**: plain, 2-D, ranged (**inclusive of both endpoints**, confirmed
  directly against NV5 Geospatial's *Array Subscript Ranges* reference),
  strided, wildcard (`*`), and negative-from-end. A single subscript
  indexes the array's own flat, column-major storage directly (both IDL
  and `array-runtime` are column-major, MA12 §2 — no translation needed);
  a 2-D subscript's axis mapping (`a[i,j]` -> column `i`, row `j`) is a
  flagged, MA12-§2-anticipated judgment call, not independently
  re-verified against a real IDL session in this cut.
- **Array literals** (`[1, 2, 3]`) — always a genuine rank-1 array (MA12
  §2's own "a scalar is genuinely rank-0, distinct from a 1-element array"
  rule), even for a one-element literal.
- **Operators**: the full precedence cascade (unary at the same tier as
  binary `+`/`-`, `^` left-associative — both confirmed against
  `idl-parser`'s own verified precedence table), `#`/`##` matrix product
  (`##` = standard `matmul`, confirmed against multiple sources; `#` =
  operand-swapped `matmul`, a flagged, moderate-confidence derivation from
  its documented shape rule, not independently re-verified against a
  primary numeric example), and `AND`/`OR`/`XOR`/`NOT` implemented as
  **bitwise** operators over an integer representation (confirmed directly
  against NV5 Geospatial's own *Bitwise Operators*/*Logical vs. Bitwise
  Operators* pages) — including the documented gotcha that bitwise `NOT`
  of a 0/1 comparison result is `-1`/`-2`, both truthy, faithfully
  reproduced rather than "fixed" into logical negation.
- **A small, explicitly scoped builtin surface** (`builtins.rs`):
  `PRINT`; the trig/math set `SIN`/`COS`/`TAN`/`SQRT`/`ABS`/`EXP`/`ALOG`/
  `ALOG10`; the `*INDGEN` family (`INDGEN`/`FINDGEN`/`DINDGEN`/`LINDGEN`,
  identical in this cut's untyped `f64` model); the `*ARR` family
  (`INTARR`/`FLTARR`/`DBLARR`/`LONARR`, 1-D and 2-D `[column, row]`
  forms); `TOTAL`/`MIN`/`MAX`/`N_ELEMENTS`; `SIZE` (four modes: the
  default dimension vector, `/N_DIMENSIONS`, `/DIMENSIONS`,
  `/N_ELEMENTS`); `TRANSPOSE`. See `builtins.rs`'s own module doc comment
  for the exact, documented reasoning behind this specific set.
- **Case folding**: every identifier (variable, routine, parameter/keyword
  name) is folded to uppercase at bind/lookup time, resolving the open
  MA-12d decision `idl-lexer`'s own README flagged — verified directly
  against NV5 Geospatial's own documented case-insensitivity (procedure
  names "converted to uppercase internally"), not guessed.
- **Display/`PRINT` convention** (`value.rs`): checked directly against
  NV5 Geospatial's *PRINT/PRINTF*/*Output of IDL Variables*/*Implied
  Print* reference pages. Verified: silent assignment, top-level-only
  auto-print, inclusive subscript ranges. Flagged as an honest judgment
  call (the official docs defer exact default numeric column
  width/precision to the platform's own `sprintf`, and note `PRINT` and
  Implied Print use genuinely different default precision from each
  other, undistinguished in this cut): plain ASCII `-`, no trailing `.0`
  for whole values, space-separated vectors, row-per-line right-aligned
  matrices — the same convention this repo's other array-family runtimes
  already use.
- DoS guards: an evaluator recursion-depth guard (`eval.rs::MAX_DEPTH`,
  500 — a disclosed, reasonable default, *not* independently re-measured
  via the same rigorous binary-search methodology `q-runtime`'s own
  `MAX_DEPTH` used, given this evaluator's own different recursion shape)
  plus `builtins::MAX_ARRAY_LENGTH` (1,000,000) capping every
  runtime-computed array allocation (construction builtins, array
  literals, subscript-range materialization) before allocating.
- 74 unit tests + 1 doc test covering: arithmetic/comparison/logical
  precedence (including the unary-at-additive-tier and left-associative
  `^` divergences from Scilab/MATLAB), the bitwise `AND`/`OR`/`XOR`/`NOT`
  semantics (including the documented `NOT` gotcha), string literals and
  `EQ`/`NE` equality, every control-flow construct (`IF`/`FOR`/`WHILE`/
  `REPEAT`/`BREAK`/`CONTINUE`, including a `BREAK`-outside-a-loop error),
  `PRO`/`FUNCTION` definitions with positional args, keyword args bound to
  a differently-spelled local, the `/BOOLEAN` shorthand, the omitted-
  keyword `N_ELEMENTS` idiom, mixed positional/keyword/boolean-shorthand
  calls, the same-name-both-a-PRO-and-a-FUNCTION two-namespace behavior,
  undefined-procedure/undefined-function/unknown-keyword errors, routine
  bodies not seeing caller/global scope, the full subscript surface
  (plain/negative/ranged/strided/wildcard/2-D) including subscripted
  assignment and out-of-range errors, array construction/reduction
  builtins, `SIZE`'s four modes, `TRANSPOSE`, Implied Print, the
  rank-1-not-scalar array-literal rule, and case folding for both
  variables and routine names.

### Fixed (security — caught by review before this release ever shipped)

- **`resolve_index`**: the subscript bounds check was written as an "out of
  range" disjunction, `idx_f < 0.0 || idx_f >= axis_len as f64`. IEEE-754
  comparisons against `NaN` are always `false`, so a `NaN` subscript
  (`a[SQRT(-1)]`, `a[0.0/0.0]`) made both disjuncts `false`, skipped the
  bounds check entirely, and fell through to `NaN as usize` (Rust's
  saturating float-to-int cast, which returns `0`) as though it were a
  validated in-bounds index — even against a zero-length axis. Indexing an
  empty array at the resulting "index 0" then panicked
  (`index out of bounds: the len is 0 but the index is 0`), uncaught
  anywhere between `resolve_index` and the `idl` binary's process boundary:
  an unauthenticated, two-line-of-input crash, reachable via ordinary IDL
  syntax with no injection/escape needed. Fixed by writing the check as the
  negated IN-RANGE condition, `!(idx_f >= 0.0 && idx_f < axis_len as f64)`,
  which is `true` for `NaN` (both `&&` operands are `false`) and so
  correctly rejects it. New regression test
  `nan_subscript_is_a_clean_error_not_a_panic` (`lib.rs`) covers the 1-D,
  2-D, and range-subscript-endpoint paths, and reproduces the exact panic
  with the fix reverted before confirming the fix resolves it. The
  `range_subscript_positions` stride path was independently audited and
  found already safe — its separate `stride == 0` check incidentally
  catches a `NaN` stride via the same truncating `as i64` cast.

### Notes

- No `array-runtime` substrate changes were needed (MA12 §2's own "zero new
  substrate" finding, confirmed directly against `ops.rs`'s current public
  API) — every arithmetic/comparison operator is either
  `ops::elementwise`/`ops::matmul`/`ops::transpose`/`ops::sum`/`ops::min`/
  `ops::max` wearing IDL's own spelling, or hand-rolled logic local to this
  crate (`^`, `AND`/`OR`/`XOR`/`NOT`, negation) for operators
  `array-runtime` has no `BinOp` variant for.
- `idl-lexer` and `idl-parser` were **not** modified.
