# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-07-20

### Added

- Initial release — **MA-10d** of the Scilab frontend: a tree-walking
  evaluator over `array-runtime` (spec
  [`MA10`](../../../specs/MA10-scilab-language.md)).
- `ScilabValue::{Num(Array), Str(String)}` — its own value enum, deliberately
  not `matlab_runtime::MatValue` (MA10 §2), with no operator implemented over
  `Str` at all beyond `==`/`~=`/`<>` equality.
- `Interpreter` (persistent workspace + registered functions) + `feed`/`eval`
  entry points producing the `name = value` prompt echo (`;` suppresses).
- Matrix/range literals, the full operator precedence cascade
  (`+ - .* ./ .\ \ * ^ .^ ' .'`, comparisons, `& | && ||`), 1-based indexing
  including Scilab's own `$`/`$-1` last-index token.
- Control flow: `if/elseif/else/end`, Scilab's own `select/case/else/end`
  multi-way conditional, `while/end`, `for/end`, `break`/`continue` — all
  correctly handling the optional `then`/`do` linker keyword `scilab-parser`'s
  `stmt_sep` production threads through six header sites.
- User-defined `function [y1,...,yn] = f(...) ... endfunction`, with a fresh
  per-call workspace and multiple return values via `[a, b] = f(x)`.
- The eight `%`-prefixed special constants (`%pi %e %inf %nan %eps %t %f` as
  ordinary numeric scalars; `%i` a clean `Err`, since complex numbers are
  deferred and `array-runtime` has no complex representation).
- The core builtin set: `zeros`/`ones`/`eye`/`size`/`length`/`numel`/`sum`/
  `mean`/`max`/`min`/`abs`/`sqrt`/`transpose`/`disp`.
- **Robustness**: `feed` runs on a dedicated worker thread inside
  `catch_unwind` with a 512 MiB stack (following `maple-runtime`'s pattern,
  not `matlab-runtime`'s older one), rebuilding the session on a panic. Flat
  operator chains evaluate via a plain iterative loop (confirmed against
  `matlab-runtime`'s identical shape, so no separate token-count guard is
  needed); a dedicated `MAX_DEPTH` bounds both nested-expression recursion and
  the genuinely new (to this crate) recursive-function-call vector; range
  length and constructor dimensions are capped (`1<<26`).
