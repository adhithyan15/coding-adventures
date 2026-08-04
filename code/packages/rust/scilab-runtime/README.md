# Scilab Runtime

A tree-walking evaluator for the [Scilab](https://www.scilab.org/) language, over
[`array-runtime`](../array-runtime). Item **MA-10d** of the Scilab frontend (spec
[`MA10`](../../../specs/MA10-scilab-language.md)): the piece that makes the
Scilab lexer/parser (MA-10b/MA-10c) executable.

## Why not `matlab-runtime`?

Scilab forks MATLAB's *grammar shape* (matrix literals, ranges, the operator
precedence cascade, indexing) but not its *semantics* — most decisively, `+`
means numeric addition on strings in MATLAB and concatenation in Scilab (MA10
§1 finding 1). Reusing `matlab_runtime::MatValue` would silently reuse
MATLAB's answer to "what does an operator mean on this variant," so this
crate has its own value enum (`ScilabValue::{Num(Array), Str(String)}`) and no
dependency on `matlab-runtime` at all. What *does* transfer unchanged: the
entire `array-runtime` numeric core (MA10 §5 — "zero substrate work").

```rust
use coding_adventures_scilab_runtime::eval;
let out = eval("A = [1 2; 3 4];\nsum(A(:, 1))\n").unwrap();
assert!(out.contains('4'));
```

## What it evaluates

- Scalars, **matrix literals** `[1 2; 3 4]`, **ranges** `a:b`/`a:b:c`.
- Operators: `+ - .* ./ .\ \ * ^ .^` (element-wise/matrix, via
  `array_runtime::ops`/`execute`), unary `+ - ~`, transpose `'`/`.'`,
  comparisons (`== ~= <> < <= > >=`), logical `& | && ||`.
- **Variables and assignment**, with `;`-suppressed display.
- **1-based indexing**: `A(i)`, `A(i,j)`, `A(:,k)`, `A(i,:)`, `A(:)`, and
  Scilab's own `$`/`$-1` last-index token (MA10 §1 finding 5 — resolved as
  "the last valid index along the current indexing dimension", not MATLAB's
  context-sensitive `end`).
- **Control flow**: `if/elseif/else/end`, Scilab's own `select/case/else/end`
  multi-way conditional (evaluate `select`'s expression once, run the first
  `case` whose expression is *equal* to it, else `else`, else nothing),
  `while/end`, `for/end` — every one with the optional `then`/`do` linker
  keyword or a bare comma/newline (already collapsed to one tree shape by
  `scilab-parser`'s own `stmt_sep`, so the runtime needs no special-casing per
  spelling), plus `break`/`continue`.
- **Functions**: `function [y1,...,yn] = f(x1,...,xm) ... endfunction`, with a
  fresh workspace per call (no closures/shared scope) and multiple return
  values via `[a, b] = f(x)`.
- **The eight `%`-prefixed special constants**: `%pi %e %inf %nan %eps %t %f`
  (ordinary numeric scalars); `%i` is a clean `Err` — complex numbers are
  deferred (MA10 §4), and `array-runtime` has no complex representation to
  substitute one from.
- **Strings** (`'...'`/`"..."`, the same type): assignment, display, and
  `==`/`~=`/`<>` equality only — **no `+` or any other operator over strings**
  (MA10 §4's explicit scope cut; see `value.rs`/`eval.rs` for where this
  absence is enforced structurally, not just by omission).
- **Builtins**: `zeros`, `ones`, `eye`, `size`, `length`, `numel`, `sum`,
  `mean`, `max`, `min`, `abs`, `sqrt`, `transpose`, `disp`.

### Deferred (documented)

`list`/`tlist`/`mlist` (Scilab's own aggregate-type system), cell-literal
(`{...}`) evaluation, complex numbers, sparse matrices, N-D arrays beyond rank
2, the Kronecker operators, `global`, indexed assignment (`A(i) = ...`), and
the wider built-in function library — the same class of deferrals
`matlab-runtime` makes for MATLAB's own first cut, applied here for Scilab
(MA10 §4).

## Robustness

- `Interpreter::feed` bounds input size, runs parsing and evaluation on a
  dedicated worker thread with a 512 MiB stack inside `catch_unwind`, and
  rebuilds the session on a panic — following `maple-runtime`'s more rigorous
  pattern rather than `matlab-runtime`'s older, simpler one. See the crate
  doc comment ("Robustness at the trust boundary") for the full accounting of
  every closed vector, including one genuinely new to this crate: recursive
  *function calls* at runtime (`eval::MAX_DEPTH`).
- Flat operator chains (`1+1+1+...`) evaluate via a **plain iterative loop**,
  not a recursive fold — confirmed against `matlab-runtime`'s identical
  pattern, so no separate token-count guard is needed for that vector.
- Range length and constructor dimensions are capped (`1<<26`), and
  `while`/recursion depth have bounds, so `1:1e18`, `zeros(1e18)`, and
  runaway loops/recursion are clean errors.

## Testing

```sh
cargo test -p coding-adventures-scilab-runtime
```
