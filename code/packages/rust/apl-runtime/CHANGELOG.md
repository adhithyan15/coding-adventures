# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial release — **MA-4e** of the APL frontend: a tree-walking evaluator
  over `array-runtime` (spec [`MA05`](../../../specs/MA05-apl-language.md)).
- `Interpreter` (persistent workspace) + `feed`/`eval` entry points. Auto-print
  semantics: an assignment is silent, a bare `value_expr` result auto-prints
  (real APL session behavior, not MATLAB's `;`-suppression).
- Right-to-left evaluation with no precedence cascade, falling straight out
  of the grammar's right-recursive `value_expr` shape — no precedence
  climbing anywhere in this crate.
- All 12 `BinOp`-mappable primitive atoms (`+ - × ÷ ⌈ ⌊ = ≠ < ≤ ≥ >`), each
  with its documented monadic meaning where one exists (conjugate, negate,
  sign, reciprocal, ceiling, floor — the six comparisons have no monadic
  form, a clean error) and its dyadic `ops::elementwise` meaning.
- `⍴`/`⍳`/`,` — bespoke monadic+dyadic primitives (shape/reshape,
  index-generator/index-of, ravel/catenate) that do not fit the
  scalar-dyadic-function mould, in their own `builtins.rs` module. Monadic
  ravel and dyadic reshape both correctly translate between APL's row-major
  convention and `array_runtime::Array`'s column-major backing store.
- `/` (reduce), `\` (scan), `∘.` (outer product) lowered onto
  `array_runtime::ops::{reduce, scan, outer}` — reduce/scan are inherently
  monadic derived functions, outer product is inherently dyadic; applying one
  in the wrong arity, or stacking an operator onto `⍴`/`⍳`/`,`, is a clean
  scope error rather than a silent guess.
- APL-style display (`value.rs`): high-minus `¯` for negatives (never ASCII
  `-`), no trailing `.0` on whole-valued floats, no `name =`/`ans =` prefix,
  space-separated vectors, right-aligned matrix rows.
- DoS guards: an independent recursion-depth guard in the evaluator (defense
  in depth on top of `apl-parser`'s own already-bounded CST), and a
  `MAX_ARRAY_LENGTH` (1,000,000) cap on `⍳n` and dyadic `⍴`'s target element
  count, checked *before* allocating.
