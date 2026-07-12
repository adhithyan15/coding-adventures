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
  `MAX_ARRAY_LENGTH` (1,000,000) cap on every primitive whose output size or
  work is driven by runtime-computed values — monadic `⍳n`, dyadic `⍴`'s
  target element count, dyadic `,`'s combined output length, `∘.`'s
  `len(a)×len(b)` output size, and dyadic `⍳`'s `len(a)×len(b)` work — every
  one checked *before* allocating or scanning, not after.

### Fixed (found by `/security-review`, before this crate's first push)

- **HIGH — dyadic `,` (catenate) had no output-size cap at all.** Unlike
  every other primitive, catenate's output can be *larger* than either
  input, so `A←A,A` doubled `A`'s size every line with no ceiling — a ~30-line
  script could reach a multi-terabyte allocation attempt. Fixed by checking
  `a.len() + b.len()` against `MAX_ARRAY_LENGTH` before allocating.
- **HIGH — `∘.` (outer product) had no output-size cap.** `ops::outer`'s own
  `checked_mul` only guards `usize` overflow, not an excessive-but-
  representable product — two individually-legal 1,000,000-element vectors
  fed to `∘.×` would request a 10^12-element (~8 TB) allocation. Fixed by
  checking `len(a) × len(b)` against `MAX_ARRAY_LENGTH` in `apply_dyadic`
  before calling into `array_runtime::ops::outer`.
- **MEDIUM — dyadic `⍳` (index-of) was O(len(a) × len(b)) with no complexity
  cap.** Two individually-legal 1,000,000-element operands could drive ~10^12
  scalar comparisons — a CPU-time hang, not a memory crash, but the same
  class of availability DoS. Fixed by checking the same product against
  `MAX_ARRAY_LENGTH` before scanning.
- All three fixes verified adversarially (guard disabled → confirmed the
  corresponding regression test fails without it → restored → confirmed it
  passes), per this repo's standing DoS-guard-verification discipline.
